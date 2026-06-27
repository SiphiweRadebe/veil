use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::fs;

use crate::utils::{db_path, ensure_veil_dir};

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

pub fn setup_team(name: &str, remote_type: &str, remote_url: &str) -> Result<()> {
    ensure_veil_dir()?;
    let conn = open_db()?;

    conn.execute(
        "INSERT OR REPLACE INTO team_settings (team_name, remote_type, remote_url, last_sync)
         VALUES (?1, ?2, ?3, ?4)",
        [
            name,
            remote_type,
            remote_url,
            &chrono::Utc::now().to_rfc3339(),
        ],
    )?;

    println!(
        "{} {} team \"{}\"",
        "veil".purple().bold(),
        "setup".green(),
        name.cyan()
    );
    println!("  {} {}", "remote:".dimmed(), remote_url.white());
    println!("  {} {}", "type:".dimmed(), remote_type.cyan());

    Ok(())
}

pub fn share_bookmark(name: &str, description: &str) -> Result<()> {
    ensure_veil_dir()?;

    // Get bookmark from local bookmarks
    let bookmarks_conn = Connection::open(db_path("bookmarks"))?;
    let path: String = bookmarks_conn.query_row(
        "SELECT path FROM bookmarks WHERE name = ?1",
        [name],
        |row| row.get(0),
    )?;

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let shared_by = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO shared_bookmarks (bookmark_name, shared_by, path, description, shared_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [name, &shared_by, &path, description, &timestamp],
    )?;

    println!(
        "{} {} bookmark \"{}\"",
        "veil".purple().bold(),
        "shared".green(),
        name.cyan()
    );
    println!("  {} {}", "by:".dimmed(), shared_by.cyan());
    println!("  {} {}", "path:".dimmed(), path.white());
    println!("  {} run: `veil team pull` to sync", "sync:".dimmed());

    Ok(())
}

pub fn share_workflow(name: &str, description: &str) -> Result<()> {
    ensure_veil_dir()?;

    // Get workflow from local patterns
    let patterns_conn = Connection::open(db_path("contextual_patterns"))?;
    let pattern_sequence: String = patterns_conn.query_row(
        "SELECT pattern_sequence FROM patterns WHERE pattern_name = ?1",
        [name],
        |row| row.get(0),
    )?;

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let shared_by = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO shared_workflows (workflow_name, shared_by, pattern_sequence, description, shared_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [name, &shared_by, &pattern_sequence, description, &timestamp],
    )?;

    println!(
        "{} {} workflow \"{}\"",
        "veil".purple().bold(),
        "shared".green(),
        name.cyan()
    );
    println!("  {} {}", "by:".dimmed(), shared_by.cyan());
    println!("  {} run: `veil team pull` to sync", "sync:".dimmed());

    Ok(())
}

pub fn team_pull() -> Result<()> {
    let conn = open_db()?;

    // Get team settings
    let mut stmt = conn.prepare("SELECT id, team_name, remote_type, remote_url FROM team_settings")?;
    let teams: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if teams.is_empty() {
        println!(
            "{} {} run: `veil team setup <name> <type> <url>`",
            "veil".purple().bold(),
            "no teams configured.".dimmed()
        );
        return Ok(());
    }

    for (_, team_name, _, _) in teams {
        println!(
            "{} {} from team \"{}\"",
            "veil".purple().bold(),
            "pulling".white(),
            team_name.cyan()
        );

        // Get shared bookmarks
        let mut bookmark_stmt = conn.prepare(
            "SELECT bookmark_name, path, shared_by FROM shared_bookmarks ORDER BY shared_at DESC LIMIT 5",
        )?;

        let bookmarks: Vec<_> = bookmark_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        if !bookmarks.is_empty() {
            println!("  {} bookmarks:", "synced".green());
            for (name, _path, by) in bookmarks {
                println!("    {} {} (from {})", "→".dimmed(), name.cyan(), by.dimmed());
            }
        }

        // Get shared workflows
        let mut workflow_stmt = conn.prepare(
            "SELECT workflow_name, shared_by FROM shared_workflows ORDER BY shared_at DESC LIMIT 5",
        )?;

        let workflows: Vec<_> = workflow_stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        if !workflows.is_empty() {
            println!("  {} workflows:", "synced".green());
            for (name, by) in workflows {
                println!("    {} {} (from {})", "→".dimmed(), name.cyan(), by.dimmed());
            }
        }
    }

    println!();
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
        println!("  {} {}", "●".green(), name.cyan().bold());
        println!("    {} {}", "type:".dimmed(), remote_type.white());
        println!("    {} {}", "url:".dimmed(), url.white());
        if let Some(sync) = last_sync {
            println!("    {} {}", "sync:".dimmed(), sync.cyan());
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
}
