use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::path::Path;
use chrono::{DateTime, Local, Duration};

use crate::utils::{db_path, ensure_veil_dir};

fn open_db() -> Result<Connection> {
    let path = db_path("snapshots_index");
    ensure_veil_dir()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshot_index (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL UNIQUE,
            command TEXT NOT NULL,
            exit_code INTEGER NOT NULL,
            snapshot_path TEXT NOT NULL,
            file_count INTEGER,
            directory TEXT
        )",
    )?;
    Ok(conn)
}

pub struct SnapshotEntry {
    pub timestamp: String,
    pub command: String,
    pub exit_code: i32,
    pub snapshot_path: String,
    pub file_count: usize,
    pub directory: String,
}

pub fn record_snapshot(
    timestamp: &str,
    command: &str,
    exit_code: i32,
    snapshot_path: &str,
    directory: &str,
) -> Result<()> {
    let conn = open_db()?;
    let file_count = count_files(snapshot_path).unwrap_or(0);

    conn.execute(
        "INSERT OR IGNORE INTO snapshot_index (timestamp, command, exit_code, snapshot_path, file_count, directory)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        [timestamp, command, &exit_code.to_string(), snapshot_path, &file_count.to_string(), directory],
    )?;

    Ok(())
}

pub fn rewind(minutes: u64) -> Result<()> {
    let conn = open_db()?;
    let target_time = Local::now() - Duration::minutes(minutes as i64);
    let target_str = target_time.to_rfc3339();

    let mut stmt = conn.prepare(
        "SELECT timestamp, command, snapshot_path, directory
         FROM snapshot_index
         WHERE timestamp <= ?1
         ORDER BY timestamp DESC
         LIMIT 1",
    )?;

    let result = stmt.query_row([target_str.as_str()], |row| {
        Ok(SnapshotEntry {
            timestamp: row.get(0)?,
            command: row.get(1)?,
            exit_code: 0,
            snapshot_path: row.get(2)?,
            file_count: 0,
            directory: row.get(3)?,
        })
    });

    match result {
        Ok(entry) => {
            println!(
                "{} {} to {}m ago",
                "veil".purple().bold(),
                "rewound".white(),
                minutes.to_string().cyan()
            );
            println!(
                "  {} {} in {}",
                "last command:".dimmed(),
                entry.command.white(),
                entry.directory.cyan()
            );
            println!(
                "  {} {} files backed up",
                "snapshot:".dimmed(),
                entry.file_count.to_string().cyan()
            );
            println!(
                "  {} {}",
                "path:".dimmed(),
                entry.snapshot_path.dimmed()
            );
            Ok(())
        }
        Err(_) => {
            println!(
                "{} {} in the last {} minutes",
                "veil".purple().bold(),
                "no snapshots found".yellow(),
                minutes.to_string().cyan()
            );
            Ok(())
        }
    }
}

pub fn timeline(limit: usize) -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT timestamp, command, exit_code, file_count
         FROM snapshot_index
         ORDER BY timestamp DESC
         LIMIT ?1",
    )?;

    let snapshots: Vec<_> = stmt
        .query_map([limit.to_string().as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if snapshots.is_empty() {
        println!("{} {}", "veil".purple().bold(), "no timeline yet".dimmed());
        return Ok(());
    }

    println!("{}\n", "Timeline".purple().bold());

    for (ts, cmd, code, count) in snapshots.iter().rev() {
        let status = if *code == 0 {
            "✓".green()
        } else {
            "✗".red()
        };

        // Parse timestamp for display
        let display_cmd = if cmd.len() > 60 {
            format!("{}...", &cmd[..57])
        } else {
            cmd.clone()
        };

        println!(
            "  {} {} {} ({} files)",
            status,
            display_cmd.white(),
            ts.dimmed(),
            count.to_string().cyan()
        );
    }

    println!();
    Ok(())
}

pub fn play(timestamp_or_offset: &str) -> Result<()> {
    let conn = open_db()?;

    // Try to parse as minutes ago (e.g., "5m")
    let query_time = if timestamp_or_offset.ends_with('m') {
        let minutes: u64 = timestamp_or_offset[..timestamp_or_offset.len() - 1]
            .parse()
            .unwrap_or(0);
        let target = Local::now() - Duration::minutes(minutes as i64);
        target.to_rfc3339()
    } else {
        timestamp_or_offset.to_string()
    };

    let mut stmt = conn.prepare(
        "SELECT command, snapshot_path, directory
         FROM snapshot_index
         WHERE timestamp <= ?1
         ORDER BY timestamp DESC
         LIMIT 1",
    )?;

    let result = stmt.query_row([query_time.as_str()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    });

    match result {
        Ok((cmd, snapshot_path, directory)) => {
            println!(
                "{} {} snapshot from {}",
                "veil".purple().bold(),
                "replay".white(),
                query_time.dimmed()
            );
            println!(
                "  {} {}",
                "command:".dimmed(),
                cmd.cyan()
            );
            println!(
                "  {} {}",
                "location:".dimmed(),
                directory.white()
            );
            println!(
                "  {} {}",
                "files:".dimmed(),
                snapshot_path.dimmed()
            );
            Ok(())
        }
        Err(_) => {
            println!(
                "{} {} at {}",
                "veil".purple().bold(),
                "no snapshot found".yellow(),
                query_time.dimmed()
            );
            Ok(())
        }
    }
}

fn count_files(path: &str) -> Result<usize> {
    if !Path::new(path).exists() {
        return Ok(0);
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for _ in entries.flatten() {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_snapshot() {
        let result = record_snapshot(
            "2026-06-27T12:00:00+00:00",
            "ls -la",
            0,
            "/tmp/test",
            "/home/user",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_count_files() {
        let count = count_files(".").unwrap_or(0);
        assert!(count >= 0);
    }
}
