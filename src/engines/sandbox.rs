use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::engines::patterns::command_danger::{assess_command_risk, format_risk};
use crate::utils::{veil_dir, ensure_veil_dir, shell_exec};

fn sandboxes_dir() -> PathBuf {
    veil_dir().join("sandboxes")
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "sandbox_{}_{}",
        duration.as_secs(),
        duration.subsec_millis()
    )
}

pub fn sandbox(cmd: &str) -> Result<()> {
    ensure_veil_dir()?;

    let session_id = generate_session_id();
    let sandbox_path = sandboxes_dir().join(&session_id);
    fs::create_dir_all(&sandbox_path)?;

    let cwd = std::env::current_dir()?;
    copy_relevant_files(&cwd, &sandbox_path, 3)?;

    // Snapshot state before running
    let before = collect_file_hashes(&sandbox_path);

    println!(
        "{} {} in {}",
        "veil".purple().bold(),
        "sandbox created".green(),
        sandbox_path.display().to_string().cyan()
    );
    println!("  {} {}", "command:".dimmed(), cmd.white());

    let output = shell_exec(cmd)
        .current_dir(&sandbox_path)
        .output();

    match output {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(-1);
            let status = if exit_code == 0 { "✓".green() } else { "✗".red() };
            println!("  {} exit code: {}", status, exit_code.to_string().cyan());

            if !output.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("\n{}", stdout.trim());
            }
            if !output.stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("{}", stderr.trim().red());
            }

            // Show what actually changed
            let (created, modified, deleted) = diff_sandbox(&sandbox_path, &before);
            println!();

            if created.is_empty() && modified.is_empty() && deleted.is_empty() {
                println!("  {} no file changes detected", "diff:".dimmed());
            } else {
                println!("  {}", "Changes in sandbox:".purple().bold());
                for f in &created {
                    println!("    {} {}", "+".green(), f.green());
                }
                for f in &modified {
                    println!("    {} {}", "~".yellow(), f.yellow());
                }
                for f in &deleted {
                    println!("    {} {}", "-".red(), f.red());
                }
            }

            println!();
            println!(
                "  {} {}",
                "sandbox path:".dimmed(),
                sandbox_path.display().to_string().dimmed()
            );
            println!("  {} rm -rf {}", "cleanup:".dimmed(), sandbox_path.display());

            Ok(())
        }
        Err(e) => {
            println!("{} {}", "error:".red(), e.to_string().white());
            Ok(())
        }
    }
}

pub fn whatif(cmd: &str) -> Result<()> {
    let (risk_level, reason) = assess_command_risk(cmd);
    let risk_emoji = format_risk(&risk_level);

    println!(
        "{} {} command analysis:",
        "veil".purple().bold(),
        "whatif".white()
    );
    println!(
        "  {} {} — {}",
        risk_emoji,
        format!("[{}]", reason).cyan(),
        cmd.white()
    );

    match risk_level {
        crate::engines::patterns::command_danger::RiskLevel::High => {
            println!(
                "\n  {} This command will likely cause permanent changes.",
                "⚠".red()
            );
            println!(
                "  {} Consider: `veil sandbox \"{}\"` first",
                "suggest:".yellow(),
                cmd
            );
        }
        crate::engines::patterns::command_danger::RiskLevel::Medium => {
            println!(
                "\n  {} This command modifies state. Review carefully.",
                "ℹ".yellow()
            );
        }
        crate::engines::patterns::command_danger::RiskLevel::Safe => {
            println!("\n  {} This appears safe to run.", "✓".green());
        }
    }

    println!();
    Ok(())
}

fn collect_file_hashes(dir: &Path) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    collect_hashes_recursive(dir, dir, &mut map);
    map
}

fn collect_hashes_recursive(root: &Path, dir: &Path, map: &mut HashMap<String, u64>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let key = rel.to_string_lossy().to_string();
        if path.is_file() {
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            map.insert(key, size);
        } else if path.is_dir() {
            collect_hashes_recursive(root, &path, map);
        }
    }
}

fn diff_sandbox(
    sandbox_path: &Path,
    before: &HashMap<String, u64>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let after = collect_file_hashes(sandbox_path);
    let mut created = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for (path, size) in &after {
        match before.get(path) {
            None => created.push(path.clone()),
            Some(old_size) if *old_size != *size => modified.push(path.clone()),
            _ => {}
        }
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            deleted.push(path.clone());
        }
    }

    created.sort();
    modified.sort();
    deleted.sort();
    (created, modified, deleted)
}

fn copy_relevant_files(src: &Path, dst: &Path, max_depth: u32) -> Result<()> {
    if max_depth == 0 {
        return Ok(());
    }

    let ignore_dirs = [
        "target", "node_modules", ".git", "__pycache__",
        ".veil", "bin", "obj", "dist", "build",
    ];

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let filename = entry.file_name();

        if ignore_dirs.contains(&filename.to_string_lossy().as_ref()) {
            continue;
        }

        let dst_path = dst.join(&filename);

        if path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_relevant_files(&path, &dst_path, max_depth - 1)?;
        } else if path.is_file() {
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.len() <= 1024 * 1024 {
                    let _ = fs::copy(&path, &dst_path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_id() {
        let id = generate_session_id();
        assert!(id.starts_with("sandbox_"));
    }

    #[test]
    fn test_sandboxes_dir() {
        let dir = sandboxes_dir();
        assert!(dir.to_string_lossy().contains("sandboxes"));
    }

    #[test]
    fn test_diff_empty() {
        let dir = std::env::temp_dir();
        let before = collect_file_hashes(&dir);
        let (created, modified, deleted) = diff_sandbox(&dir, &before);
        assert!(created.is_empty());
        assert!(modified.is_empty());
        assert!(deleted.is_empty());
    }
}
