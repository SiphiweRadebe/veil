use anyhow::Result;
use colored::*;
use rusqlite::Connection;

use crate::engines::patterns::fuzzy_match::find_similar;
use crate::engines::patterns::sequence_detect::{detect_patterns, find_next_command};
use crate::utils::{db_path, ensure_veil_dir};

fn open_db() -> Result<Connection> {
    let path = db_path("contextual_patterns");
    ensure_veil_dir()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS patterns (
            id INTEGER PRIMARY KEY,
            pattern_sequence TEXT NOT NULL UNIQUE,
            count INTEGER DEFAULT 1,
            last_used TEXT,
            context_dir TEXT,
            pattern_name TEXT,
            success_rate REAL DEFAULT 1.0
        )",
    )?;
    Ok(conn)
}

/// Normalize a command so variations collapse to the same template.
/// `git commit -m "anything"` and `git commit -m "fix typo"` → `git commit -m <value>`
/// This makes pattern detection useful across real workflows.
fn normalize_command(cmd: &str) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return cmd.to_string();
    }

    let base = parts[0];
    let args = &parts[1..];
    let mut normalized: Vec<String> = Vec::new();
    let mut skip_next = false;

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            normalized.push("<value>".to_string());
            continue;
        }
        // Flags that consume the next token as a value
        if matches!(
            *arg,
            "-m" | "--message" | "-t" | "--tag" | "-o" | "--output"
            | "-f" | "--file" | "--author" | "--branch" | "--format"
        ) {
            normalized.push(arg.to_string());
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            normalized.push(arg.to_string());
            continue;
        }
        // Quoted strings → <message>
        if arg.starts_with('"') || arg.starts_with('\'') {
            normalized.push("<message>".to_string());
            continue;
        }
        // URLs
        if arg.starts_with("http") || arg.starts_with("git@") {
            normalized.push("<url>".to_string());
            continue;
        }
        // Git hashes (hex 7-40 chars)
        if arg.len() >= 7 && arg.len() <= 40 && arg.chars().all(|c| c.is_ascii_hexdigit()) {
            normalized.push("<hash>".to_string());
            continue;
        }
        // File/directory paths
        if arg.contains('/') || arg.contains('\\')
            || (arg.contains('.') && !matches!(*arg, "." | ".."))
        {
            normalized.push("<path>".to_string());
            continue;
        }
        // Version strings like "1.2.3"
        if arg.split('.').count() >= 2 && arg.chars().all(|c| c.is_ascii_digit() || c == '.') {
            normalized.push("<version>".to_string());
            continue;
        }
        // For git/cargo specifically, keep known subcommands; genericize the rest past position 1
        if i >= 1
            && !is_known_subcommand(base, args[0])
            && !arg.starts_with('-')
        {
            normalized.push("<arg>".to_string());
            continue;
        }
        normalized.push(arg.to_string());
    }

    format!("{} {}", base, normalized.join(" ")).trim().to_string()
}

fn is_known_subcommand(base: &str, sub: &str) -> bool {
    match base {
        "git" => matches!(
            sub,
            "add" | "commit" | "push" | "pull" | "clone" | "checkout" | "switch"
            | "branch" | "merge" | "rebase" | "stash" | "status" | "log"
            | "diff" | "fetch" | "reset" | "tag" | "remote" | "init"
        ),
        "cargo" => matches!(
            sub,
            "build" | "run" | "test" | "check" | "clean" | "add" | "remove"
            | "install" | "publish" | "fmt" | "clippy" | "update" | "doc"
        ),
        "npm" | "yarn" | "pnpm" => matches!(
            sub,
            "install" | "run" | "build" | "test" | "start" | "publish"
            | "add" | "remove" | "update" | "ci"
        ),
        _ => false,
    }
}

pub fn related(query: &str) -> Result<()> {
    ensure_veil_dir()?;
    let memoir_path = db_path("memoir");
    let memoir_conn = Connection::open(&memoir_path)?;

    let mut stmt = match memoir_conn.prepare(
        "SELECT command FROM commands ORDER BY timestamp DESC LIMIT 200",
    ) {
        Ok(s) => s,
        Err(_) => {
            println!("{} {} similar to \"{}\"", "veil".purple().bold(), "no commands".yellow(), query.cyan());
            return Ok(());
        }
    };

    let commands: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    // Deduplicate by normalized form so we don't show 20 variations of the same command
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<String> = commands
        .into_iter()
        .filter(|c| seen.insert(normalize_command(c)))
        .collect();

    let candidate_strs: Vec<&str> = deduped.iter().map(|s| s.as_str()).collect();
    let results = find_similar(query, &candidate_strs, 0.55);

    if results.is_empty() {
        println!(
            "{} {} similar to \"{}\"",
            "veil".purple().bold(),
            "no commands".yellow(),
            query.cyan()
        );
        return Ok(());
    }

    println!("{} {} similar commands:\n", "veil".purple().bold(), "found".green());

    for (cmd, score) in results.iter().take(7) {
        let score_percent = (score * 100.0) as u32;
        println!(
            "  {} {} ({}% match)",
            "→".dimmed(),
            cmd.white(),
            score_percent.to_string().cyan()
        );
    }

    println!();
    Ok(())
}

pub fn workflow_list() -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT pattern_name, pattern_sequence, count, success_rate FROM patterns ORDER BY count DESC",
    )?;

    let workflows: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, usize>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if workflows.is_empty() {
        println!(
            "{} {} run `veil workflow save <name>`",
            "veil".purple().bold(),
            "no workflows yet.".dimmed()
        );
        return Ok(());
    }

    println!("{}\n", "Your Workflows".purple().bold());

    for (name, sequence, count, success_rate) in workflows {
        let success_emoji = if success_rate > 0.9 {
            "✓".green()
        } else if success_rate > 0.7 {
            "◐".yellow()
        } else {
            "✗".red()
        };

        let workflow_name = name.unwrap_or_else(|| format!("workflow_{}", count));
        println!(
            "  {} {} {}× ({}% success)",
            success_emoji,
            workflow_name.cyan().bold(),
            count.to_string().dimmed(),
            ((success_rate * 100.0) as u32).to_string().cyan()
        );

        // Show the normalized sequence
        let steps: Vec<&str> = sequence.split(" | ").collect();
        for step in &steps {
            println!("    {} {}", "→".dimmed(), step.white());
        }
    }

    println!();
    Ok(())
}

pub fn workflow_save(name: &str) -> Result<()> {
    ensure_veil_dir()?;
    let memoir_path = db_path("memoir");
    let memoir_conn = Connection::open(&memoir_path)?;

    let mut stmt = match memoir_conn.prepare(
        "SELECT command, exit_code FROM commands ORDER BY timestamp DESC LIMIT 100",
    ) {
        Ok(s) => s,
        Err(_) => {
            println!("{} {} run some commands first", "veil".purple().bold(), "no history yet.".dimmed());
            return Ok(());
        }
    };

    let history: Vec<_> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // Normalize all commands before detecting patterns so variations collapse
    let normalized_history: Vec<(String, i32)> = history
        .iter()
        .map(|(cmd, code)| (normalize_command(cmd), *code))
        .collect();

    let patterns = detect_patterns(&normalized_history, 2, 1);

    if patterns.is_empty() {
        println!(
            "{} {} commands aren't repeated enough yet",
            "veil".purple().bold(),
            "no patterns found".yellow()
        );
        return Ok(());
    }

    let best_pattern = &patterns[0];
    let sequence_str = best_pattern
        .commands
        .iter()
        .map(|c| c.as_str())
        .collect::<Vec<_>>()
        .join(" | ");

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO patterns (pattern_name, pattern_sequence, count, last_used, success_rate)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            name,
            &sequence_str,
            &best_pattern.count.to_string(),
            &timestamp,
            &best_pattern.success_rate.to_string(),
        ],
    )?;

    println!(
        "{} workflow {} saved",
        "veil".purple().bold(),
        name.cyan().bold()
    );
    println!("  {} {}×", "used".dimmed(), best_pattern.count.to_string().cyan());
    println!(
        "  {} {}%",
        "success:".dimmed(),
        ((best_pattern.success_rate * 100.0) as u32).to_string().cyan()
    );

    let steps: Vec<&str> = sequence_str.split(" | ").collect();
    println!("  {} {} steps:", "steps:".dimmed(), steps.len().to_string().cyan());
    for step in &steps {
        println!("    {} {}", "→".dimmed(), step.white());
    }

    Ok(())
}

pub fn next() -> Result<()> {
    ensure_veil_dir()?;
    let memoir_path = db_path("memoir");
    let memoir_conn = Connection::open(&memoir_path)?;

    let mut stmt = match memoir_conn.prepare(
        "SELECT command, exit_code FROM commands ORDER BY timestamp DESC LIMIT 50",
    ) {
        Ok(s) => s,
        Err(_) => {
            println!("{} {} run some commands first", "veil".purple().bold(), "no history yet.".dimmed());
            return Ok(());
        }
    };

    let history: Vec<_> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    if history.is_empty() {
        println!(
            "{} {} run some commands first",
            "veil".purple().bold(),
            "no history yet.".dimmed()
        );
        return Ok(());
    }

    let last_cmd = &history[0].0;
    let normalized_last = normalize_command(last_cmd);

    // Normalize all history before lookup so we match across variations
    let normalized_history: Vec<(String, i32)> = history
        .iter()
        .map(|(cmd, code)| (normalize_command(cmd), *code))
        .collect();

    let suggestions = find_next_command(&normalized_history, &normalized_last, 5);

    if suggestions.is_empty() {
        println!(
            "{} {} what to run after \"{}\"",
            "veil".purple().bold(),
            "unsure".dimmed(),
            last_cmd.cyan()
        );
        return Ok(());
    }

    println!(
        "{} {} after \"{}\":\n",
        "veil".purple().bold(),
        "next".green(),
        last_cmd.cyan()
    );

    for (cmd, frequency) in suggestions {
        println!(
            "  {} {} ({}×)",
            "→".dimmed(),
            cmd.white(),
            frequency.to_string().cyan()
        );
    }

    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_git_commit() {
        let n1 = normalize_command("git commit -m \"fix bug\"");
        let n2 = normalize_command("git commit -m \"add feature\"");
        assert_eq!(n1, n2, "Different commit messages should normalize to same pattern");
    }

    #[test]
    fn test_normalize_preserves_flags() {
        let n = normalize_command("cargo build --release");
        assert!(n.contains("--release"));
    }

    #[test]
    fn test_normalize_path() {
        let n = normalize_command("cp src/main.rs backup/main.rs");
        assert!(n.contains("<path>"));
    }

    #[test]
    fn test_workflow_list() {
        let result = workflow_list();
        assert!(result.is_ok());
    }

    #[test]
    fn test_next() {
        let result = next();
        assert!(result.is_ok());
    }
}
