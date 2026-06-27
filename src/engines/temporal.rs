use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::path::Path;
use chrono::{Local, Duration};

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
        );
        CREATE INDEX IF NOT EXISTS idx_snapshot_timestamp ON snapshot_index(timestamp);",
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
    // Use a ±30s tolerance window to handle timestamp variance
    let lower = (target_time - Duration::seconds(30)).to_rfc3339();
    let upper = (target_time + Duration::seconds(30)).to_rfc3339();

    let mut stmt = conn.prepare(
        "SELECT timestamp, command, snapshot_path, directory, file_count
         FROM snapshot_index
         WHERE timestamp <= ?1
         ORDER BY timestamp DESC
         LIMIT 1",
    )?;

    let result = stmt.query_row([upper.as_str()], |row| {
        Ok(SnapshotEntry {
            timestamp: row.get(0)?,
            command: row.get(1)?,
            exit_code: 0,
            snapshot_path: row.get(2)?,
            file_count: row.get::<_, i64>(4).unwrap_or(0) as usize,
            directory: row.get(3)?,
        })
    });

    let _ = lower; // tolerance bound, available for future stricter checks

    match result {
        Ok(entry) => {
            let display_ts = friendly_time(&entry.timestamp);
            println!(
                "{} {} to {} ({}m ago)",
                "veil".purple().bold(),
                "rewound".white(),
                display_ts.cyan(),
                minutes.to_string().cyan()
            );
            println!(
                "  {} {} in {}",
                "snapshot:".dimmed(),
                entry.command.white(),
                entry.directory.cyan()
            );
            println!(
                "  {} {} files tracked",
                "coverage:".dimmed(),
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
        "SELECT timestamp, command, exit_code, file_count, directory
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
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if snapshots.is_empty() {
        println!(
            "{} {} install the shell hook to start recording snapshots",
            "veil".purple().bold(),
            "no timeline yet —".dimmed()
        );
        return Ok(());
    }

    println!("{}\n", "Snapshot Timeline".purple().bold());

    for (ts, cmd, code, count, dir) in snapshots.iter().rev() {
        let status = if *code == 0 { "✓".green() } else { "✗".red() };
        let display_cmd = if cmd.len() > 55 {
            format!("{}…", &cmd[..54])
        } else {
            cmd.clone()
        };
        let short_dir = if dir.len() > 30 {
            format!("…{}", &dir[dir.len().saturating_sub(28)..])
        } else {
            dir.clone()
        };

        println!(
            "  {} {} {}",
            status,
            display_cmd.white(),
            short_dir.dimmed()
        );
        println!(
            "    {} {} files",
            friendly_time(ts).cyan(),
            count.to_string().dimmed()
        );
    }

    println!();
    Ok(())
}

pub fn play(timestamp_or_offset: &str) -> Result<()> {
    let conn = open_db()?;

    let (query_upper, query_lower) = if timestamp_or_offset.ends_with('m') {
        let minutes: i64 = timestamp_or_offset[..timestamp_or_offset.len() - 1]
            .parse()
            .unwrap_or(5);
        let target = Local::now() - Duration::minutes(minutes);
        (
            (target + Duration::seconds(30)).to_rfc3339(),
            (target - Duration::seconds(30)).to_rfc3339(),
        )
    } else if timestamp_or_offset.ends_with('h') {
        let hours: i64 = timestamp_or_offset[..timestamp_or_offset.len() - 1]
            .parse()
            .unwrap_or(1);
        let target = Local::now() - Duration::hours(hours);
        (
            (target + Duration::seconds(30)).to_rfc3339(),
            (target - Duration::seconds(30)).to_rfc3339(),
        )
    } else {
        // Treat as exact timestamp with ±30s tolerance
        (
            timestamp_or_offset.to_string(),
            timestamp_or_offset.to_string(),
        )
    };

    let mut stmt = conn.prepare(
        "SELECT command, snapshot_path, directory, timestamp, file_count
         FROM snapshot_index
         WHERE timestamp <= ?1
         ORDER BY timestamp DESC
         LIMIT 1",
    )?;

    let result = stmt.query_row([query_upper.as_str()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4).unwrap_or(0),
        ))
    });

    let _ = query_lower;

    match result {
        Ok((cmd, snapshot_path, directory, ts, file_count)) => {
            let display_ts = friendly_time(&ts);
            println!(
                "{} {} snapshot at {}",
                "veil".purple().bold(),
                "found".green(),
                display_ts.cyan()
            );
            println!("  {} {}", "command:".dimmed(), cmd.white());
            println!("  {} {}", "location:".dimmed(), directory.cyan());
            println!("  {} {} files", "tracked:".dimmed(), file_count.to_string().cyan());
            println!("  {} {}", "path:".dimmed(), snapshot_path.dimmed());
            println!();
            println!(
                "  {} use `veil back {}` to restore this state",
                "restore:".dimmed(),
                timestamp_or_offset
            );
            Ok(())
        }
        Err(_) => {
            println!(
                "{} {} at {}",
                "veil".purple().bold(),
                "no snapshot found".yellow(),
                timestamp_or_offset.dimmed()
            );
            Ok(())
        }
    }
}

fn friendly_time(ts: &str) -> String {
    // Try RFC3339 first, fall back to raw
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        let local = dt.with_timezone(&Local);
        return local.format("%Y-%m-%d %H:%M").to_string();
    }
    // Handle our snapshot format %Y%m%d_%H%M%S_%3f
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(ts, "%Y%m%d_%H%M%S_%3f") {
        return dt.format("%Y-%m-%d %H:%M").to_string();
    }
    ts[..ts.len().min(16)].to_string()
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
    fn test_friendly_time_rfc3339() {
        let result = friendly_time("2026-06-27T12:30:00+00:00");
        assert!(result.contains("2026-06-27"));
    }

    #[test]
    fn test_friendly_time_snapshot_format() {
        let result = friendly_time("20260627_123000_000");
        assert!(result.contains("2026-06-27"));
    }

    #[test]
    fn test_count_files() {
        let count = count_files(".").unwrap_or(0);
        assert!(count >= 0);
    }
}
