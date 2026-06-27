use anyhow::{Result};
use colored::*;
use rusqlite::Connection;

fn db_path() -> String {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{}/.veil/sessions.db", home)
}

fn open_db() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            command TEXT NOT NULL,
            exit_code INTEGER NOT NULL,
            directory TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            sequence INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

/// Record a command in the current session
pub fn record(command: &str, exit_code: i32, directory: &str) -> Result<()> {
    if command.trim().is_empty() || command.starts_with("veil ") {
        return Ok(());
    }

    let conn = open_db()?;
    let session_id = get_or_create_session_id();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let sequence: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE session_id = ?1",
        [session_id.as_str()],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO sessions (session_id, command, exit_code, directory, timestamp, sequence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        [
            &session_id,
            command,
            &exit_code.to_string(),
            directory,
            &timestamp,
            &sequence.to_string(),
        ],
    )?;

    Ok(())
}

/// Replay the last N commands from current session with timestamps
pub fn replay(limit: usize) -> Result<()> {
    let conn = open_db()?;
    let session_id = get_or_create_session_id();

    let mut stmt = conn.prepare(
        "SELECT command, exit_code, timestamp, sequence
         FROM sessions
         WHERE session_id = ?1
         ORDER BY sequence DESC
         LIMIT ?2",
    )?;

    let commands: Vec<_> = stmt
        .query_map([session_id.as_str(), &limit.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if commands.is_empty() {
        println!("{}", "No session history yet.".dimmed());
        return Ok(());
    }

    println!("{}\n", "Session Replay".purple().bold());

    for (cmd, exit_code, ts, seq) in commands.iter().rev() {
        let status = if *exit_code == 0 {
            "✓".green()
        } else {
            "✗".red()
        };

        println!(
            "  {} {} {} {}",
            format!("#{}", seq).dimmed(),
            status,
            cmd.white(),
            ts.dimmed()
        );
    }

    println!();
    Ok(())
}

/// Get session ID, creating a new one if needed
fn get_or_create_session_id() -> String {
    std::env::var("VEIL_SESSION_ID")
        .unwrap_or_else(|_| {
            let id = chrono::Local::now().timestamp().to_string();
            std::env::set_var("VEIL_SESSION_ID", &id);
            id
        })
}
