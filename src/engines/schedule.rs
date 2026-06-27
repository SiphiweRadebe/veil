use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::process::Command;
use std::thread;
use std::time::Duration;
use chrono::Local;

use crate::utils::{db_path, ensure_veil_dir};

fn open_db() -> Result<Connection> {
    let path = db_path("scheduling");
    ensure_veil_dir()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schedules (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            cron_expression TEXT NOT NULL,
            command TEXT NOT NULL,
            last_run TEXT,
            next_run TEXT,
            enabled BOOLEAN DEFAULT 1,
            created_at TEXT
        )",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schedule_events (
            id INTEGER PRIMARY KEY,
            schedule_id INTEGER,
            ran_at TEXT,
            exit_code INTEGER,
            duration_ms INTEGER
        )",
    )?;
    Ok(conn)
}

pub fn schedule(name: &str, cron: &str, command: &str) -> Result<()> {
    ensure_veil_dir()?;

    // Validate cron expression (basic validation)
    if !is_valid_cron(cron) {
        println!(
            "{} {} invalid cron: {}",
            "veil".purple().bold(),
            "error".red(),
            cron.yellow()
        );
        println!("  {} format: minute hour day month dow",
            "expected:".dimmed()
        );
        println!("  {} examples: \"0 9 * * *\" (daily at 9am)",
            "".dimmed()
        );
        return Ok(());
    }

    let conn = open_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let next_run = calculate_next_run(cron)?;

    conn.execute(
        "INSERT OR REPLACE INTO schedules (name, cron_expression, command, enabled, created_at, next_run)
         VALUES (?1, ?2, ?3, 1, ?4, ?5)",
        [name, cron, command, &timestamp, &next_run],
    )?;

    println!(
        "{} {} \"{}\"",
        "veil".purple().bold(),
        "scheduled".green(),
        name.cyan()
    );
    println!("  {} {}", "cron:".dimmed(), cron.white());
    println!("  {} {}", "command:".dimmed(), command.dimmed());
    println!("  {} {}", "next run:".dimmed(), next_run.cyan());

    Ok(())
}

pub fn schedule_list() -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT name, cron_expression, command, next_run, enabled FROM schedules ORDER BY created_at DESC",
    )?;

    let schedules: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if schedules.is_empty() {
        println!(
            "{} {} run: `veil schedule <name> <cron> <cmd>`",
            "veil".purple().bold(),
            "no schedules yet.".dimmed()
        );
        return Ok(());
    }

    println!("{}\n", "Scheduled Tasks".purple().bold());

    for (name, cron, cmd, next_run, enabled) in schedules {
        let status = if enabled { "●".green() } else { "○".dimmed() };
        println!("  {} {}", status, name.cyan().bold());
        println!("    {} {}", "cron:".dimmed(), cron.white());
        println!("    {} {}", "next:".dimmed(), next_run.unwrap_or_else(|| "pending".to_string()).cyan());
        println!("    {} {}\n", "cmd:".dimmed(), cmd.dimmed());
    }

    Ok(())
}

pub fn schedule_run(name: &str) -> Result<()> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, command, cron_expression FROM schedules WHERE name = ?1 AND enabled = 1",
    )?;

    let schedule = stmt.query_row([name], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    });

    match schedule {
        Ok((schedule_id, command, cron)) => {
            println!(
                "{} {} running \"{}\"",
                "veil".purple().bold(),
                "schedule".green(),
                name.cyan()
            );
            println!("  {} {}", "cron:".dimmed(), cron.white());

            loop {
                let next_run = calculate_next_run(&cron)?;
                let now = Local::now().to_rfc3339();

                // Simple check: run if next_run is in the past
                if next_run <= now {
                    let start = std::time::Instant::now();
                    let output = Command::new("sh").arg("-c").arg(&command).output();

                    if let Ok(output) = output {
                        let duration = start.elapsed().as_millis();
                        let exit_code = output.status.code().unwrap_or(-1);
                        let timestamp = chrono::Utc::now().to_rfc3339();

                        let _ = conn.execute(
                            "INSERT INTO schedule_events (schedule_id, ran_at, exit_code, duration_ms)
                             VALUES (?1, ?2, ?3, ?4)",
                            [
                                &schedule_id.to_string(),
                                &timestamp,
                                &exit_code.to_string(),
                                &duration.to_string(),
                            ],
                        );

                        let status = if exit_code == 0 {
                            "✓".green()
                        } else {
                            "✗".red()
                        };

                        println!(
                            "  {} {} ({}ms)",
                            status,
                            timestamp.dimmed(),
                            duration.to_string().cyan()
                        );
                    }

                    // Update next run time
                    let new_next = calculate_next_run(&cron)?;
                    let _ = conn.execute(
                        "UPDATE schedules SET next_run = ?1 WHERE id = ?2",
                        [new_next.as_str(), &schedule_id.to_string()],
                    );
                }

                // Check every minute
                thread::sleep(Duration::from_secs(60));
            }
        }
        Err(_) => {
            println!(
                "{} {} schedule \"{}\"",
                "veil".purple().bold(),
                "not found".yellow(),
                name.cyan()
            );
        }
    }

    Ok(())
}

pub fn schedule_remove(name: &str) -> Result<()> {
    let conn = open_db()?;
    conn.execute("UPDATE schedules SET enabled = 0 WHERE name = ?1", [name])?;

    println!(
        "{} {} schedule \"{}\"",
        "veil".purple().bold(),
        "disabled".yellow(),
        name.cyan()
    );

    Ok(())
}

fn is_valid_cron(expr: &str) -> bool {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    // Basic validation: should have 5 parts (minute hour day month dow)
    parts.len() == 5
        && parts.iter().all(|p| {
            *p == "*" || p.parse::<u32>().is_ok() || p.contains('-') || p.contains(',')
        })
}

fn calculate_next_run(_cron: &str) -> Result<String> {
    // Simplified: just add time until next occurrence
    // In production, use the `cron` crate for proper calculation
    let now = Local::now();
    let next = now + chrono::Duration::hours(1);
    Ok(next.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_cron() {
        assert!(is_valid_cron("0 9 * * *"));
        assert!(is_valid_cron("*/5 * * * *"));
        assert!(!is_valid_cron("invalid"));
        assert!(!is_valid_cron("0 9 *"));
    }

    #[test]
    fn test_schedule_list() {
        let result = schedule_list();
        assert!(result.is_ok());
    }
}
