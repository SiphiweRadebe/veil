use anyhow::Result;
use colored::*;
use std::collections::HashMap;

/// Capture current environment variables
pub fn capture() -> Result<()> {
    let env_snapshot = std::env::vars().collect::<HashMap<_, _>>();
    let path = env_snapshot_path();

    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&env_snapshot)?;
    std::fs::write(&path, json)?;

    println!(
        "{} {} environment variables captured",
        "veil".purple().bold(),
        env_snapshot.len().to_string().cyan()
    );
    Ok(())
}

/// Compare environment variables and warn about changes
pub fn diff() -> Result<()> {
    let path = env_snapshot_path();

    if !std::path::Path::new(&path).exists() {
        println!(
            "{} {} — run `veil env capture` first",
            "veil".purple().bold(),
            "no baseline found".yellow()
        );
        return Ok(());
    }

    let stored_json = std::fs::read_to_string(&path)?;
    let stored: HashMap<String, String> = serde_json::from_str(&stored_json)?;
    let current: HashMap<String, String> = std::env::vars().collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (key, current_val) in &current {
        match stored.get(key) {
            None => added.push(key.clone()),
            Some(stored_val) if stored_val != current_val => {
                changed.push((key.clone(), stored_val.clone(), current_val.clone()));
            }
            _ => {}
        }
    }

    for key in stored.keys() {
        if !current.contains_key(key) {
            removed.push(key.clone());
        }
    }

    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        println!(
            "{} {} — no changes detected",
            "veil".purple().bold(),
            "environment clean".green()
        );
        return Ok(());
    }

    println!("{} {} changes detected:\n", "veil".purple().bold(), (added.len() + removed.len() + changed.len()).to_string().yellow());

    if !added.is_empty() {
        println!("  {} {} variables added:", "→".green(), added.len());
        for key in added.iter().take(5) {
            println!("    {}", key.green());
        }
        if added.len() > 5 {
            println!("    {} more...", added.len() - 5);
        }
    }

    if !removed.is_empty() {
        println!("  {} {} variables removed:", "→".red(), removed.len());
        for key in removed.iter().take(5) {
            println!("    {}", key.red());
        }
        if removed.len() > 5 {
            println!("    {} more...", removed.len() - 5);
        }
    }

    if !changed.is_empty() {
        println!("  {} {} variables changed:", "→".yellow(), changed.len());
        for (key, old_val, new_val) in changed.iter().take(3) {
            println!("    {} {} {}",
                key.yellow(),
                "→".dimmed(),
                format!("{} → {}",
                    if old_val.len() > 30 { format!("{}...", &old_val[..27]) } else { old_val.clone() },
                    if new_val.len() > 30 { format!("{}...", &new_val[..27]) } else { new_val.clone() }
                ).cyan()
            );
        }
        if changed.len() > 3 {
            println!("    {} more...", changed.len() - 3);
        }
    }

    println!();
    Ok(())
}

fn env_snapshot_path() -> String {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{}/.veil/env_snapshot.json", home)
}
