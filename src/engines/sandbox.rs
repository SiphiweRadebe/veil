use anyhow::Result;
use colored::*;
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
        (duration.subsec_millis() % 1000)
    )
}

pub fn sandbox(cmd: &str) -> Result<()> {
    ensure_veil_dir()?;

    let session_id = generate_session_id();
    let sandbox_path = sandboxes_dir().join(&session_id);
    fs::create_dir_all(&sandbox_path)?;

    // Copy current directory structure (limited to avoid huge copies)
    let cwd = std::env::current_dir()?;
    copy_relevant_files(&cwd, &sandbox_path, 3)?;

    println!(
        "{} {} in {}",
        "veil".purple().bold(),
        "sandbox created".green(),
        sandbox_path.display().to_string().cyan()
    );
    println!(
        "  {} {}",
        "command:".dimmed(),
        cmd.white()
    );

    // Run command in sandbox
    let output = shell_exec(cmd)
        .current_dir(&sandbox_path)
        .output();

    match output {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(-1);
            let status = if exit_code == 0 {
                "✓".green()
            } else {
                "✗".red()
            };

            println!(
                "  {} exit code: {}",
                status,
                exit_code.to_string().cyan()
            );

            // Analyze what changed
            let changes = count_changes(&sandbox_path)?;
            if changes > 0 {
                println!(
                    "  {} {} files/directories",
                    "changes:".yellow(),
                    changes.to_string().cyan()
                );
            }

            println!(
                "\n  {} {}",
                "sandbox path:".dimmed(),
                sandbox_path.display().to_string().dimmed()
            );
            println!("  {} rm -rf {}\n", "cleanup:".dimmed(), sandbox_path.display());

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
    println!("  {} {} — {}",
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
            // Skip large files
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.len() > 1024 * 1024 {
                    // Skip files > 1MB
                    continue;
                }
            }
            let _ = fs::copy(&path, &dst_path);
        }
    }

    Ok(())
}

fn count_changes(sandbox_path: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(sandbox_path)? {
        if entry.is_ok() {
            count += 1;
        }
    }
    Ok(count)
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
}
