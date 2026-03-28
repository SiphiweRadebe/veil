use anyhow::Result;
use rusqlite::{Connection, params};
use chrono::Local;
use colored::*;

fn db_path() -> String {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{}/.veil/memoir.db", home)
}

fn open_db() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS commands (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            command     TEXT NOT NULL,
            directory   TEXT NOT NULL,
            exit_code   INTEGER NOT NULL,
            timestamp   TEXT NOT NULL
        );
    ")?;
    Ok(conn)
}

pub fn record(command: &str, exit_code: i32, directory: &str) -> Result<()> {
    if command.trim().is_empty() || command.starts_with("veil ") {
        return Ok(());
    }
    let conn = open_db()?;
    let timestamp = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO commands (command, directory, exit_code, timestamp) VALUES (?1, ?2, ?3, ?4)",
        params![command, directory, exit_code, timestamp],
    )?;
    Ok(())
}

pub fn find(query: &str) -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT command, directory, exit_code, timestamp
         FROM commands
         WHERE command LIKE ?1
         ORDER BY timestamp DESC
         LIMIT 20"
    )?;
    let pattern = format!("%{}%", query);
    let results: Vec<(String, String, i32, String)> = stmt.query_map(
        params![pattern],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    )?.filter_map(|r| r.ok()).collect();

    if results.is_empty() {
        println!("{}", "No commands found.".dimmed());
        return Ok(());
    }

    println!("{} results for {}\n", results.len().to_string().purple(), format!("\"{}\"", query).white().bold());

    for (cmd, dir, exit_code, timestamp) in &results {
        let status = if *exit_code == 0 { "✓".green() } else { "✗".red() };
        let short_time = &timestamp[..16].replace("T", " ");
        println!("  {} {}", status, cmd.white().bold());
        println!("    {} {}  {}", "in".dimmed(), dir.cyan(), short_time.dimmed());
        println!();
    }
    Ok(())
}