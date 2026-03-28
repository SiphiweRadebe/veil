use anyhow::Result;
use colored::*;
use std::path::Path;

pub fn status() -> Result<()> {
    let cwd = std::env::current_dir()?;
    println!("{}\n", "Project status".purple().bold());
    println!("  {} {}", "stack".dimmed(), detect_project(&cwd).cyan());
    println!("  {} {}", "directory".dimmed(), cwd.display().to_string().white());
    if Path::new(".git").exists() {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("  {} {}", "branch".dimmed(), branch.yellow());
        }
    }
    Ok(())
}

fn detect_project(dir: &std::path::Path) -> String {
    if dir.join("Cargo.toml").exists()       { return "Rust".to_string(); }
    if dir.join("package.json").exists()     { return "Node.js".to_string(); }
    if dir.join("requirements.txt").exists() { return "Python".to_string(); }
    if dir.join("go.mod").exists()           { return "Go".to_string(); }
    "Unknown".to_string()
}