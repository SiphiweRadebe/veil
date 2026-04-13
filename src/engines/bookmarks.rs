use anyhow::{Result, anyhow};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default)]
struct Bookmarks {
    entries: HashMap<String, String>,
}

fn bookmarks_path() -> String {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{}/.veil/bookmarks.json", home)
}

fn load() -> Result<Bookmarks> {
    let path = bookmarks_path();
    if !std::path::Path::new(&path).exists() {
        return Ok(Bookmarks::default());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save(bookmarks: &Bookmarks) -> Result<()> {
    let path = bookmarks_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(bookmarks)?)?;
    Ok(())
}

/// Add a bookmark — saves current directory under a name
pub fn add(name: &str, directory: &str) -> Result<()> {
    let mut bookmarks = load()?;
    bookmarks.entries.insert(name.to_string(), directory.to_string());
    save(&bookmarks)?;
    println!(
        "{} {} {} {}",
        "veil".purple().bold(),
        "bookmarked".green(),
        name.white().bold(),
        format!("→ {}", directory).dimmed()
    );
    Ok(())
}

/// Get the path for a bookmark by name
pub fn get(name: &str) -> Result<String> {
    let bookmarks = load()?;
    bookmarks.entries.get(name)
        .cloned()
        .ok_or_else(|| anyhow!(
            "No bookmark named '{}'. Run `veil bookmark add {}` to create it.",
            name, name
        ))
}

/// Remove a bookmark
pub fn remove(name: &str) -> Result<()> {
    let mut bookmarks = load()?;
    if bookmarks.entries.remove(name).is_none() {
        println!("{}", format!("No bookmark named '{}'.", name).yellow());
        return Ok(());
    }
    save(&bookmarks)?;
    println!("{} removed bookmark {}", "veil".purple().bold(), name.white().bold());
    Ok(())
}

/// List all bookmarks
pub fn list() -> Result<()> {
    let bookmarks = load()?;

    if bookmarks.entries.is_empty() {
        println!("{}", "No bookmarks yet.".dimmed());
        println!("{}", "Run `veil bookmark add <name>` to save a directory.".dimmed());
        return Ok(());
    }

    println!("{}\n", "Bookmarks".purple().bold());

    let mut entries: Vec<(&String, &String)> = bookmarks.entries.iter().collect();
    entries.sort_by_key(|(k, _)| k.as_str());

    for (name, path) in entries {
        println!("  {} {}", format!("{}", name).white().bold(), format!("→ {}", path).dimmed());
    }

    println!();
    println!("{}", "Use `veil go <name>` to jump to a bookmark.".dimmed());

    Ok(())
}
