use anyhow::Result;
use colored::*;
use rusqlite::Connection;

use crate::utils::{db_path, ensure_veil_dir};

fn open_db() -> Result<Connection> {
    let path = db_path("remotes");
    ensure_veil_dir()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS remote_hosts (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            host TEXT NOT NULL,
            user TEXT,
            key_path TEXT,
            tags TEXT,
            last_connected TEXT
        )",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS remote_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL UNIQUE,
            shared_by TEXT,
            shared_at TEXT,
            expires_at TEXT,
            access_token TEXT,
            view_only BOOLEAN DEFAULT 0
        )",
    )?;
    Ok(conn)
}

pub fn add_host(
    name: &str,
    host: &str,
    user: &str,
    key_path: Option<&str>,
    tags: Option<&str>,
) -> Result<()> {
    ensure_veil_dir()?;
    let conn = open_db()?;

    conn.execute(
        "INSERT OR REPLACE INTO remote_hosts (name, host, user, key_path, tags)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [name, host, user, key_path.unwrap_or(""), tags.unwrap_or("")],
    )?;

    println!("{} {} host \"{}\"", "veil".purple().bold(), "added".green(), name.cyan());
    println!("  {} {}", "host:".dimmed(), host.white());
    println!("  {} {}", "user:".dimmed(), user.cyan());
    if let Some(key) = key_path {
        println!("  {} {}", "key:".dimmed(), key.dimmed());
    }

    Ok(())
}

pub fn host_list() -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT name, host, user, tags, last_connected FROM remote_hosts ORDER BY name",
    )?;

    let hosts: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if hosts.is_empty() {
        println!(
            "{} {} add: `veil remote add <name> <host> <user>`",
            "veil".purple().bold(),
            "no hosts yet.".dimmed()
        );
        return Ok(());
    }

    println!("{}\n", "Remote Hosts".purple().bold());

    for (name, host, user, tags, last) in hosts {
        let status = if last.is_some() { "●".green() } else { "○".dimmed() };
        println!("  {} {}", status, name.cyan().bold());
        println!("    {} {}@{}", "address:".dimmed(), user.white(), host.cyan());
        if let Some(t) = tags {
            println!("    {} {}", "tags:".dimmed(), t.dimmed());
        }
        if let Some(l) = last {
            println!("    {} {}", "last:".dimmed(), l.dimmed());
        }
        println!();
    }

    Ok(())
}

pub fn ssh(host: &str, command: &str) -> Result<()> {
    let conn = open_db()?;

    let (user, hostname): (String, String) = conn.query_row(
        "SELECT user, host FROM remote_hosts WHERE name = ?1",
        [host],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    println!(
        "{} {} \"{}\" on {}",
        "veil".purple().bold(),
        "executing".white(),
        command.cyan(),
        host.cyan()
    );

    let remote_cmd = format!("ssh {}@{} \"{}\"", user, hostname, command);
    let _ = &remote_cmd; // kept for logging context
    let output = std::process::Command::new("ssh")
        .arg(format!("{}@{}", user, hostname))
        .arg(command)
        .output();

    match output {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(-1);
            let status = if exit_code == 0 {
                "✓".green()
            } else {
                "✗".red()
            };

            println!("  {} exit code: {}", status, exit_code.to_string().cyan());

            if !output.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("\n{}\n", stdout);
            }

            if !output.stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("{}", stderr.red());
            }

            // Update last_connected
            let timestamp = chrono::Utc::now().to_rfc3339();
            let _ = conn.execute(
                "UPDATE remote_hosts SET last_connected = ?1 WHERE name = ?2",
                [&timestamp, host],
            );

            Ok(())
        }
        Err(e) => {
            println!("{} {}", "error:".red(), e.to_string().white());
            Ok(())
        }
    }
}

pub fn broadcast(pattern: &str, command: &str) -> Result<()> {
    let conn = open_db()?;

    // Find hosts matching pattern
    let mut stmt = conn.prepare(
        "SELECT name, user, host FROM remote_hosts WHERE name LIKE ?1 OR tags LIKE ?1 ORDER BY name",
    )?;

    let pattern_sql = format!("%{}%", pattern);
    let hosts: Vec<_> = stmt
        .query_map([pattern_sql.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if hosts.is_empty() {
        println!(
            "{} {} matching \"{}\"",
            "veil".purple().bold(),
            "no hosts".yellow(),
            pattern.cyan()
        );
        return Ok(());
    }

    println!(
        "{} {} to {} hosts:\n",
        "veil".purple().bold(),
        "broadcasting".white(),
        hosts.len().to_string().cyan()
    );

    for (_name, user, host) in hosts {
        println!("  {} {}@{}", "→".dimmed(), user.cyan(), host.white());

        let remote_cmd = format!("ssh {}@{} \"{}\"", user, host, command);
        let _ = &remote_cmd;
        let output = std::process::Command::new("ssh")
            .arg(format!("{}@{}", user, host))
            .arg(command)
            .output();

        match output {
            Ok(output) => {
                let exit_code = output.status.code().unwrap_or(-1);
                let status = if exit_code == 0 {
                    "✓".green()
                } else {
                    "✗".red()
                };
                println!("    {} (exit: {})", status, exit_code.to_string().cyan());
            }
            Err(e) => {
                println!("    {} {}", "✗".red(), e.to_string().dimmed());
            }
        }
    }

    println!();
    Ok(())
}

pub fn replay_share(session_id: &str) -> Result<()> {
    ensure_veil_dir()?;

    // Read session from recorder.db
    let sessions_conn = Connection::open(db_path("sessions"))?;
    let _exists: i32 = sessions_conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?;

    if _exists == 0 {
        println!(
            "{} {} session \"{}\"",
            "veil".purple().bold(),
            "not found".yellow(),
            session_id.cyan()
        );
        return Ok(());
    }

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let access_token = format!("token_{}", now.as_secs());
    let expires = chrono::Utc::now() + chrono::Duration::days(7);
    let expires_str = expires.to_rfc3339();

    conn.execute(
        "INSERT INTO remote_sessions (session_id, shared_by, shared_at, expires_at, access_token)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            session_id,
            &std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
            &timestamp,
            &expires_str,
            &access_token,
        ],
    )?;

    println!(
        "{} {} session",
        "veil".purple().bold(),
        "shared".green()
    );
    println!(
        "  {} {}",
        "session:".dimmed(),
        session_id.cyan()
    );
    println!(
        "  {} {}",
        "token:".dimmed(),
        access_token.white()
    );
    println!(
        "  {} {}",
        "expires:".dimmed(),
        expires_str.dimmed()
    );
    println!(
        "\n  {} veil replay-show {} -t {}",
        "view:".dimmed(),
        session_id.cyan(),
        access_token.dimmed()
    );

    Ok(())
}

pub fn host_remove(name: &str) -> Result<()> {
    let conn = open_db()?;
    conn.execute("DELETE FROM remote_hosts WHERE name = ?1", [name])?;

    println!(
        "{} {} host \"{}\"",
        "veil".purple().bold(),
        "removed".yellow(),
        name.cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_list() {
        let result = host_list();
        assert!(result.is_ok());
    }
}
