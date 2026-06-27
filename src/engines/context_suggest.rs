use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::collections::HashMap;

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

pub fn related(query: &str) -> Result<()> {
    // Read command history from memoir.db
    let memoir_path = crate::utils::db_path("memoir");
    let memoir_conn = Connection::open(&memoir_path)?;

    let mut stmt = memoir_conn.prepare(
        "SELECT command FROM commands ORDER BY timestamp DESC LIMIT 100",
    )?;

    let commands: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let candidate_strs: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    let results = find_similar(query, &candidate_strs, 0.6);

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

    for (cmd, score) in results.iter().take(5) {
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
            "no workflows yet. ".dimmed()
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

        let sequence_display = if sequence.len() > 80 {
            format!("{}...", &sequence[..77])
        } else {
            sequence
        };
        println!("    {}", sequence_display.dimmed());
    }

    println!();
    Ok(())
}

pub fn workflow_save(name: &str) -> Result<()> {
    // Read recent commands from memoir
    let memoir_path = crate::utils::db_path("memoir");
    let memoir_conn = Connection::open(&memoir_path)?;

    let mut stmt = memoir_conn.prepare(
        "SELECT command, exit_code FROM commands ORDER BY timestamp DESC LIMIT 50",
    )?;

    let history: Vec<_> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // Detect patterns
    let patterns = detect_patterns(&history, 2, 1);

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

    Ok(())
}

pub fn next() -> Result<()> {
    // Read recent commands from memoir
    let memoir_path = crate::utils::db_path("memoir");
    let memoir_conn = Connection::open(&memoir_path)?;

    let mut stmt = memoir_conn.prepare(
        "SELECT command, exit_code FROM commands ORDER BY timestamp DESC LIMIT 20",
    )?;

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
    let suggestions = find_next_command(&history, last_cmd, 3);

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
            "  {} {} ({}× times)",
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
    fn test_workflow_list() {
        // Test should handle empty database gracefully
        let result = workflow_list();
        assert!(result.is_ok());
    }

    #[test]
    fn test_next() {
        let result = next();
        assert!(result.is_ok());
    }
}
