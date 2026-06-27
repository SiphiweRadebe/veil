use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::thread;
use std::time::Duration;

use crate::utils::{db_path, ensure_veil_dir, shell_exec};

fn open_db() -> Result<Connection> {
    let path = db_path("monitoring");
    ensure_veil_dir()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS watchers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            pattern TEXT NOT NULL,
            command TEXT NOT NULL,
            enabled BOOLEAN DEFAULT 1,
            created_at TEXT
        )",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS watch_events (
            id INTEGER PRIMARY KEY,
            watcher_id INTEGER,
            changed_files TEXT,
            command_run TEXT,
            exit_code INTEGER,
            timestamp TEXT
        )",
    )?;
    Ok(conn)
}

pub fn watch(name: &str, pattern: &str, command: &str) -> Result<()> {
    ensure_veil_dir()?;
    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO watchers (name, pattern, command, enabled, created_at)
         VALUES (?1, ?2, ?3, 1, ?4)",
        [name, pattern, command, &timestamp],
    )?;

    println!(
        "{} {} watching \"{}\"",
        "veil".purple().bold(),
        "watch".green(),
        pattern.cyan()
    );
    println!("  {} {}", "command:".dimmed(), command.white());
    println!(
        "  {} run: `veil watch-run {}`",
        "start:".dimmed(),
        name.cyan()
    );

    Ok(())
}

pub fn watch_list() -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, pattern, command, enabled FROM watchers ORDER BY created_at DESC",
    )?;

    let watchers: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if watchers.is_empty() {
        println!(
            "{} {} run: `veil watch <name> <pattern> <cmd>`",
            "veil".purple().bold(),
            "no watchers yet.".dimmed()
        );
        return Ok(());
    }

    println!("{}\n", "Active Watchers".purple().bold());

    for (_, name, pattern, cmd, enabled) in watchers {
        let status = if enabled { "●".green() } else { "○".dimmed() };
        println!("  {} {}", status, name.cyan().bold());
        println!("    {} {}", "pattern:".dimmed(), pattern.white());
        println!("    {} {}\n", "command:".dimmed(), cmd.dimmed());
    }

    Ok(())
}

pub fn watch_run(name: &str, poll_interval: u64) -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, pattern, command FROM watchers WHERE name = ?1 AND enabled = 1",
    )?;

    let watcher = stmt.query_row([name], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    });

    match watcher {
        Ok((watcher_id, _pattern, command)) => {
            println!(
                "{} {} watching for changes...",
                "veil".purple().bold(),
                "monitor".green()
            );
            println!("  {} to stop (Ctrl+C)", "press".dimmed());

            loop {
                // In a real implementation, use the notify crate for actual file watching
                // For now, simulate by checking at intervals
                thread::sleep(Duration::from_secs(poll_interval));

                // Run the command
                let output = shell_exec(&command).output();

                if let Ok(output) = output {
                    let exit_code = output.status.code().unwrap_or(-1);
                    let timestamp = chrono::Utc::now().to_rfc3339();

                    let _ = conn.execute(
                        "INSERT INTO watch_events (watcher_id, command_run, exit_code, timestamp)
                         VALUES (?1, ?2, ?3, ?4)",
                        [
                            &watcher_id.to_string(),
                            &command,
                            &exit_code.to_string(),
                            &timestamp,
                        ],
                    );

                    let status = if exit_code == 0 {
                        "✓".green()
                    } else {
                        "✗".red()
                    };

                    println!(
                        "  {} {} (exit: {})",
                        status,
                        timestamp.dimmed(),
                        exit_code.to_string().cyan()
                    );
                }
            }
        }
        Err(_) => {
            println!(
                "{} {} watcher \"{}\"",
                "veil".purple().bold(),
                "not found".yellow(),
                name.cyan()
            );
        }
    }

    Ok(())
}

pub fn watch_remove(name: &str) -> Result<()> {
    let conn = open_db()?;
    conn.execute("UPDATE watchers SET enabled = 0 WHERE name = ?1", [name])?;

    println!(
        "{} {} watcher \"{}\"",
        "veil".purple().bold(),
        "disabled".yellow(),
        name.cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_list() {
        let result = watch_list();
        assert!(result.is_ok());
    }
}
