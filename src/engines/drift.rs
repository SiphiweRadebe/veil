use anyhow::{Result, anyhow};
use colored::*;
use std::{fs, path::{Path, PathBuf}};
use chrono::Local;

fn snapshot_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}/.veil/snapshots", home))
}

pub fn snapshot(command: &str, directory: &str) -> Result<()> {
    let snap_dir = snapshot_dir();
    fs::create_dir_all(&snap_dir)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let snap_path = snap_dir.join(&timestamp);
    fs::create_dir_all(&snap_path)?;

    let meta = serde_json::json!({
        "command": command,
        "directory": directory,
        "timestamp": timestamp,
    });
    fs::write(snap_path.join("meta.json"), meta.to_string())?;

    let files_path = snap_path.join("files");
    copy_dir(Path::new(directory), &files_path, 0)?;

    // Keep snapshots lean: remove anything older than 7 days
    cleanup_old_snapshots();

    Ok(())
}

fn copy_dir(src: &Path, dst: &Path, depth: u32) -> Result<()> {
    if depth > 3 { return Ok(()); }
    if !src.exists() { return Ok(()); }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let src_path = entry.path();
        let dst_path = dst.join(&name);

        if matches!(name_str.as_ref(),
            "target" | "node_modules" | ".git" |
            "__pycache__" | ".veil" | "bin" | "obj"
        ) {
            continue;
        }

        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path, depth + 1)?;
        } else if src_path.is_file() {
            if let Ok(meta) = src_path.metadata() {
                if meta.len() < 1_000_000 {
                    fs::copy(&src_path, &dst_path)?;
                }
            }
        }
    }
    Ok(())
}

fn cleanup_old_snapshots() {
    let snap_dir = snapshot_dir();
    let cutoff = Local::now() - chrono::Duration::days(7);

    let Ok(entries) = fs::read_dir(&snap_dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let meta_file = path.join("meta.json");
        let Ok(content) = fs::read_to_string(&meta_file) else { continue };
        let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) else { continue };
        let Some(ts) = meta["timestamp"].as_str() else { continue };
        let Ok(t) = chrono::NaiveDateTime::parse_from_str(ts, "%Y%m%d_%H%M%S_%3f") else { continue };
        if t.and_utc() < cutoff {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

pub fn undo() -> Result<()> {
    let snap_dir = snapshot_dir();

    if !snap_dir.exists() {
        println!("{}", "Nothing to undo — veil hasn't recorded any snapshots yet.".yellow());
        println!("{}", "Run a few commands first so veil can track them.".dimmed());
        return Ok(());
    }

    let mut snapshots: Vec<PathBuf> = fs::read_dir(&snap_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    snapshots.sort();

    if snapshots.len() < 2 {
        println!("{}", "Nothing to undo yet — need at least 2 snapshots.".yellow());
        return Ok(());
    }

    let target = &snapshots[snapshots.len() - 2];
    let latest = &snapshots[snapshots.len() - 1];

    let meta: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(latest.join("meta.json"))?
    )?;

    let command = meta["command"].as_str().unwrap_or("unknown");
    let directory = meta["directory"].as_str().unwrap_or(".");

    println!("{} undoing: {}", "veil".purple().bold(), command.white().bold());
    println!("{} restoring {} ...", "→".dimmed(), directory.cyan());

    restore_dir(&target.join("files"), Path::new(directory))?;
    fs::remove_dir_all(latest)?;

    println!("{}", "Done. Your files are back to where they were.".green());
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

pub fn go_back(minutes: u64) -> Result<()> {
    let snap_dir = snapshot_dir();
    let cutoff = Local::now() - chrono::Duration::minutes(minutes as i64);

    let mut snapshots: Vec<PathBuf> = fs::read_dir(&snap_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    snapshots.sort();

    let target = snapshots.iter().rev().find(|p| {
        if let Ok(content) = fs::read_to_string(p.join("meta.json")) {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(ts) = meta["timestamp"].as_str() {
                    if let Ok(t) = chrono::NaiveDateTime::parse_from_str(ts, "%Y%m%d_%H%M%S_%3f") {
                        return t.and_utc() < cutoff;
                    }
                }
            }
        }
        false
    });

    match target {
        Some(snap) => {
            let meta: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(snap.join("meta.json"))?
            )?;
            let directory = meta["directory"].as_str().unwrap_or(".");
            let command = meta["command"].as_str().unwrap_or("unknown");
            println!(
                "{} rolling back {}m → before \"{}\"",
                "veil".purple().bold(),
                minutes,
                command.white()
            );
            restore_dir(&snap.join("files"), Path::new(directory))?;
            println!("{}", "Done.".green());
        }
        None => {
            return Err(anyhow!("No snapshot found from {} minutes ago.", minutes));
        }
    }
    Ok(())
}
