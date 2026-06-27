use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::collections::HashMap;

fn db_path() -> String {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{}/.veil/aliases.db", home)
}

fn open_db() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS aliases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            alias TEXT NOT NULL UNIQUE,
            command TEXT NOT NULL,
            created_at TEXT NOT NULL,
            usage_count INTEGER DEFAULT 0
        )",
        [],
    )?;
    Ok(conn)
}

/// Add a custom alias
pub fn add_alias(alias: &str, command: &str) -> Result<()> {
    if alias.len() < 2 {
        return Err(anyhow::anyhow!("Alias must be at least 2 characters"));
    }

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO aliases (alias, command, created_at)
         VALUES (?1, ?2, ?3)",
        [alias, command, &timestamp],
    )?;

    println!(
        "{} aliased {} {} {}",
        "veil".purple().bold(),
        alias.cyan().bold(),
        "→".dimmed(),
        command.white()
    );
    Ok(())
}

/// Get an alias
pub fn get_alias(alias: &str) -> Result<String> {
    let conn = open_db()?;
    let command = conn.query_row(
        "SELECT command FROM aliases WHERE alias = ?1",
        [alias],
        |row| row.get::<_, String>(0),
    )?;

    // Increment usage count
    let _ = conn.execute(
        "UPDATE aliases SET usage_count = usage_count + 1 WHERE alias = ?1",
        [alias],
    );

    Ok(command)
}

/// List all aliases
pub fn list_aliases() -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT alias, command, usage_count FROM aliases ORDER BY usage_count DESC",
    )?;

    let aliases: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if aliases.is_empty() {
        println!("{}", "No aliases yet. Run `veil alias add <name> <command>`.".dimmed());
        return Ok(());
    }

    println!("{}\n", "Your Aliases".purple().bold());
    for (alias, cmd, count) in aliases {
        println!(
            "  {} {} {} {}",
            alias.cyan().bold(),
            "→".dimmed(),
            cmd.white(),
            format!("({}×)", count).dimmed()
        );
    }
    println!();
    Ok(())
}

/// Suggest aliases based on command history patterns
pub fn suggest() -> Result<()> {
    // This will analyze memoir.db for repeating patterns
    let memoir_path = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let memoir_path = format!("{}/.veil/memoir.db", memoir_path);

    if !std::path::Path::new(&memoir_path).exists() {
        println!("{}", "Need command history first. Run some commands!".dimmed());
        return Ok(());
    }

    let conn = Connection::open(&memoir_path)?;
    let mut stmt = conn.prepare(
        "SELECT command FROM commands ORDER BY timestamp DESC LIMIT 100",
    )?;

    let commands: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut patterns: HashMap<String, i32> = HashMap::new();

    // Find long commands that appear multiple times
    for cmd in commands {
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        if tokens.len() >= 2 && cmd.len() > 15 {
            *patterns.entry(cmd.clone()).or_insert(0) += 1;
        }
    }

    let mut suggestions: Vec<_> = patterns
        .iter()
        .filter(|(_, count)| **count >= 2)
        .collect();

    suggestions.sort_by_key(|(_, count)| -(**count));

    if suggestions.is_empty() {
        println!(
            "{} {} commands aren't repeated enough yet to suggest aliases",
            "veil".purple().bold(),
            "your".dimmed()
        );
        return Ok(());
    }

    println!("{} {} potential aliases:\n", "veil".purple().bold(), "suggesting".green());

    for (cmd, count) in suggestions.iter().take(5) {
        let short = if cmd.len() > 50 {
            format!("{}...", &cmd[..47])
        } else {
            cmd.to_string()
        };

        println!("  {} (used {}×)", short.white(), count.to_string().cyan());
        println!(
            "    {} `veil alias add <name> {}`\n",
            "run:".dimmed(),
            cmd.dimmed()
        );
    }

    Ok(())
}
