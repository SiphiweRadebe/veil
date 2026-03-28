use anyhow::{Result, anyhow};
use colored::*;
use std::{fs, path::{Path, PathBuf}};

fn snapshot_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}/.veil/snapshots", home))
}

pub fn undo() -> Result<()> {
    let snap_dir = snapshot_dir();
    if !snap_dir.exists() {
        println!("{}", "Nothing to undo — no snapshots found.".yellow());
        return Ok(());
    }
    let mut snapshots: Vec<PathBuf> = fs::read_dir(&snap_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    snapshots.sort();
    if snapshots.len() < 2 {
        println!("{}", "Nothing to undo.".yellow());
        return Ok(());
    }
    let target = &snapshots[snapshots.len() - 2];
    let meta: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(target.join("meta.json"))?
    )?;
    let command = meta["command"].as_str().unwrap_or("unknown");
    let directory = meta["directory"].as_str().unwrap_or(".");
    println!("{} undoing: {}", "veil".purple().bold(), command.white().bold());
    restore_dir(&target.join("files"), Path::new(directory))?;
    fs::remove_dir_all(&snapshots[snapshots.len() - 1])?;
    println!("{}", "Done. Restored to previous state.".green());
    Ok(())
}

fn restore_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() { return Ok(()); }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            restore_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub fn go_back(_minutes: u64) -> Result<()> {
    println!("{}", "veil back — coming in v0.2".dimmed());
    Err(anyhow!("Not yet implemented"))
}