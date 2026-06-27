use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::fs;
use std::process::Command;

use crate::utils::{db_path, ensure_veil_dir, veil_dir};

fn open_db() -> Result<Connection> {
    let path = db_path("team_sync");
    ensure_veil_dir()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS team_settings (
            id INTEGER PRIMARY KEY,
            team_name TEXT NOT NULL UNIQUE,
            remote_type TEXT NOT NULL,
            remote_url TEXT NOT NULL,
            auth_token TEXT,
            last_sync TEXT
        )",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS shared_bookmarks (
            id INTEGER PRIMARY KEY,
            bookmark_name TEXT NOT NULL UNIQUE,
            shared_by TEXT,
            path TEXT NOT NULL,
            description TEXT,
            shared_at TEXT,
            team_id INTEGER
        )",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS shared_workflows (
            id INTEGER PRIMARY KEY,
            workflow_name TEXT NOT NULL UNIQUE,
            shared_by TEXT,
            pattern_sequence TEXT NOT NULL,
            description TEXT,
            shared_at TEXT,
            team_id INTEGER
        )",
    )?;
    Ok(conn)
}

fn team_cache_dir(team_name: &str) -> std::path::PathBuf {
    veil_dir().join("team-cache").join(team_name)
}

pub fn setup_team(name: &str, remote_type: &str, remote_url: &str) -> Result<()> {
    ensure_veil_dir()?;
    let conn = open_db()?;

    conn.execute(
        "INSERT OR REPLACE INTO team_settings (team_name, remote_type, remote_url, last_sync)
         VALUES (?1, ?2, ?3, ?4)",
        [name, remote_type, remote_url, &chrono::Utc::now().to_rfc3339()],
    )?;

    println!(
        "{} {} team \"{}\"",
        "veil".purple().bold(),
        "setup".green(),
        name.cyan()
    );
    println!("  {} {}", "remote:".dimmed(), remote_url.white());
    println!("  {} {}", "type:".dimmed(), remote_type.cyan());
    println!("  {} run `veil team pull` to sync", "next:".dimmed());

    Ok(())
}

pub fn share_bookmark(name: &str, description: &str) -> Result<()> {
    ensure_veil_dir()?;

    let bookmarks_conn = Connection::open(db_path("bookmarks"))?;
    let path: String = bookmarks_conn.query_row(
        "SELECT path FROM bookmarks WHERE name = ?1",
        [name],
        |row| row.get(0),
    )?;

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let shared_by = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO shared_bookmarks (bookmark_name, shared_by, path, description, shared_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [name, &shared_by, &path, description, &timestamp],
    )?;

    // Export to team cache if a git remote is configured
    push_to_remote(&conn, "bookmark", name)?;

    println!(
        "{} {} bookmark \"{}\"",
        "veil".purple().bold(),
        "shared".green(),
        name.cyan()
    );
    println!("  {} {}", "by:".dimmed(), shared_by.cyan());
    println!("  {} {}", "path:".dimmed(), path.white());

    Ok(())
}

pub fn share_workflow(name: &str, description: &str) -> Result<()> {
    ensure_veil_dir()?;

    let patterns_conn = Connection::open(db_path("contextual_patterns"))?;
    let pattern_sequence: String = patterns_conn.query_row(
        "SELECT pattern_sequence FROM patterns WHERE pattern_name = ?1",
        [name],
        |row| row.get(0),
    )?;

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let shared_by = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO shared_workflows (workflow_name, shared_by, pattern_sequence, description, shared_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [name, &shared_by, &pattern_sequence, description, &timestamp],
    )?;

    push_to_remote(&conn, "workflow", name)?;

    println!(
        "{} {} workflow \"{}\"",
        "veil".purple().bold(),
        "shared".green(),
        name.cyan()
    );
    println!("  {} {}", "by:".dimmed(), shared_by.cyan());
    println!("  {} {}", "steps:".dimmed(), pattern_sequence.white());

    Ok(())
}

fn push_to_remote(conn: &Connection, _kind: &str, _name: &str) -> Result<()> {
    let team: Option<(String, String, String)> = conn.query_row(
        "SELECT team_name, remote_type, remote_url FROM team_settings LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).ok();

    let Some((team_name, remote_type, _remote_url)) = team else { return Ok(()); };
    if remote_type != "github" && remote_type != "git" { return Ok(()); }

    let cache_dir = team_cache_dir(&team_name);
    if cache_dir.join(".git").exists() {
        // Export current state and commit
        export_team_json(conn, &cache_dir)?;
        let _ = Command::new("git").args(["add", "."]).current_dir(&cache_dir).output();
        let msg = format!("veil: update shared config");
        let _ = Command::new("git")
            .args(["commit", "-m", &msg])
            .current_dir(&cache_dir)
            .output();
        let _ = Command::new("git").args(["push"]).current_dir(&cache_dir).output();
    }
    Ok(())
}

pub fn team_pull() -> Result<()> {
    let conn = open_db()?;

    let teams: Vec<(i32, String, String, String)> = {
        let mut stmt = conn.prepare("SELECT id, team_name, remote_type, remote_url FROM team_settings")?;
        stmt.query_map([], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?
    };

    if teams.is_empty() {
        println!(
            "{} {} run: `veil team setup <name> <type> <url>`",
            "veil".purple().bold(),
            "no teams configured.".dimmed()
        );
        return Ok(());
    }

    for (_, team_name, remote_type, remote_url) in &teams {
        println!(
            "{} {} from team \"{}\"",
            "veil".purple().bold(),
            "pulling".white(),
            team_name.cyan()
        );

        // Sync remote if git-backed
        if remote_type == "github" || remote_type == "git" {
            sync_git_remote(team_name, remote_url)?;
        }

        // Import from local cache or DB
        let cache_dir = team_cache_dir(team_name);
        if cache_dir.exists() {
            let imported = import_from_cache(&conn, &cache_dir)?;
            println!(
                "  {} {} bookmark(s), {} workflow(s) imported",
                "synced:".green(),
                imported.0.to_string().cyan(),
                imported.1.to_string().cyan()
            );
        }

        // Show what's available
        show_shared_items(&conn)?;

        // Update last_sync timestamp
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "UPDATE team_settings SET last_sync = ?1 WHERE team_name = ?2",
            [&now, team_name.as_str()],
        );
    }

    println!();
    Ok(())
}

fn sync_git_remote(team_name: &str, remote_url: &str) -> Result<()> {
    let cache_dir = team_cache_dir(team_name);

    if cache_dir.join(".git").exists() {
        // Already cloned — pull latest
        let output = Command::new("git")
            .args(["pull", "--rebase", "--autostash"])
            .current_dir(&cache_dir)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                println!("  {} up-to-date from remote", "git:".dimmed());
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                println!("  {} git pull failed: {}", "warn:".yellow(), stderr.trim().dimmed());
            }
            Err(e) => {
                println!("  {} git not available: {}", "warn:".yellow(), e.to_string().dimmed());
            }
        }
    } else {
        // First time — clone
        fs::create_dir_all(&cache_dir)?;
        println!("  {} cloning team config...", "git:".dimmed());

        let output = Command::new("git")
            .args(["clone", remote_url, "."])
            .current_dir(&cache_dir)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                println!("  {} cloned successfully", "git:".green());
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                println!("  {} clone failed: {}", "warn:".yellow(), stderr.trim().dimmed());
            }
            Err(e) => {
                println!("  {} git not available: {}", "warn:".yellow(), e.to_string().dimmed());
            }
        }
    }

    Ok(())
}

fn import_from_cache(conn: &Connection, cache_dir: &std::path::Path) -> Result<(usize, usize)> {
    let mut bookmarks_imported = 0;
    let mut workflows_imported = 0;

    // Look for veil-team.json in the cache dir
    let config_file = cache_dir.join("veil-team.json");
    if !config_file.exists() {
        return Ok((0, 0));
    }

    let content = fs::read_to_string(&config_file)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(bookmarks) = config["bookmarks"].as_array() {
        for bm in bookmarks {
            let name = bm["name"].as_str().unwrap_or_default();
            let path = bm["path"].as_str().unwrap_or_default();
            let by = bm["shared_by"].as_str().unwrap_or("team");
            let desc = bm["description"].as_str().unwrap_or("");
            if name.is_empty() || path.is_empty() { continue; }

            let res = conn.execute(
                "INSERT OR IGNORE INTO shared_bookmarks (bookmark_name, shared_by, path, description, shared_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                [name, by, path, desc, &chrono::Utc::now().to_rfc3339()],
            );
            if res.is_ok() {
                bookmarks_imported += 1;
            }
        }
    }

    if let Some(workflows) = config["workflows"].as_array() {
        for wf in workflows {
            let name = wf["name"].as_str().unwrap_or_default();
            let sequence = wf["pattern_sequence"].as_str().unwrap_or_default();
            let by = wf["shared_by"].as_str().unwrap_or("team");
            let desc = wf["description"].as_str().unwrap_or("");
            if name.is_empty() || sequence.is_empty() { continue; }

            let res = conn.execute(
                "INSERT OR IGNORE INTO shared_workflows (workflow_name, shared_by, pattern_sequence, description, shared_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                [name, by, sequence, desc, &chrono::Utc::now().to_rfc3339()],
            );
            if res.is_ok() {
                workflows_imported += 1;
            }
        }
    }

    Ok((bookmarks_imported, workflows_imported))
}

fn export_team_json(conn: &Connection, cache_dir: &std::path::Path) -> Result<()> {
    let mut bm_stmt = conn.prepare(
        "SELECT bookmark_name, path, shared_by, description FROM shared_bookmarks",
    )?;
    let bookmarks: Vec<serde_json::Value> = bm_stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "name": row.get::<_, String>(0)?,
                "path": row.get::<_, String>(1)?,
                "shared_by": row.get::<_, String>(2)?,
                "description": row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            }))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut wf_stmt = conn.prepare(
        "SELECT workflow_name, pattern_sequence, shared_by, description FROM shared_workflows",
    )?;
    let workflows: Vec<serde_json::Value> = wf_stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "name": row.get::<_, String>(0)?,
                "pattern_sequence": row.get::<_, String>(1)?,
                "shared_by": row.get::<_, String>(2)?,
                "description": row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            }))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let config = serde_json::json!({
        "bookmarks": bookmarks,
        "workflows": workflows,
        "exported_at": chrono::Utc::now().to_rfc3339(),
    });

    fs::create_dir_all(cache_dir)?;
    fs::write(cache_dir.join("veil-team.json"), serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn show_shared_items(conn: &Connection) -> Result<()> {
    let mut bm_stmt = conn.prepare(
        "SELECT bookmark_name, path, shared_by FROM shared_bookmarks ORDER BY shared_at DESC LIMIT 10",
    )?;
    let bookmarks: Vec<(String, String, String)> = bm_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    if !bookmarks.is_empty() {
        println!("  {} bookmarks:", "team".green());
        for (name, path, by) in &bookmarks {
            println!("    {} {} → {} (by {})", "→".dimmed(), name.cyan(), path.white(), by.dimmed());
        }
    }

    let mut wf_stmt = conn.prepare(
        "SELECT workflow_name, shared_by, pattern_sequence FROM shared_workflows ORDER BY shared_at DESC LIMIT 10",
    )?;
    let workflows: Vec<(String, String, String)> = wf_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    if !workflows.is_empty() {
        println!("  {} workflows:", "team".green());
        for (name, by, seq) in &workflows {
            let short_seq = if seq.len() > 60 { format!("{}…", &seq[..59]) } else { seq.clone() };
            println!("    {} {} (by {}) — {}", "→".dimmed(), name.cyan(), by.dimmed(), short_seq.white());
        }
    }

    Ok(())
}

pub fn team_list() -> Result<()> {
    let conn = open_db()?;

    let mut stmt = conn.prepare(
        "SELECT team_name, remote_type, remote_url, last_sync FROM team_settings",
    )?;

    let teams: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if teams.is_empty() {
        println!(
            "{} {} run: `veil team setup <name> <type> <url>`",
            "veil".purple().bold(),
            "no teams yet.".dimmed()
        );
        return Ok(());
    }

    println!("{}\n", "Team Configurations".purple().bold());

    for (name, remote_type, url, last_sync) in teams {
        let cache_status = if team_cache_dir(&name).join(".git").exists() {
            "●".green()
        } else {
            "○".dimmed()
        };
        println!("  {} {}", cache_status, name.cyan().bold());
        println!("    {} {}", "type:".dimmed(), remote_type.white());
        println!("    {} {}", "url:".dimmed(), url.white());
        if let Some(sync) = last_sync {
            let short = &sync[..sync.len().min(19)];
            println!("    {} {}", "last sync:".dimmed(), short.replace('T', " ").cyan());
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_list() {
        let result = team_list();
        assert!(result.is_ok());
    }

    #[test]
    fn test_team_pull_no_teams() {
        let result = team_pull();
        assert!(result.is_ok());
    }
}
