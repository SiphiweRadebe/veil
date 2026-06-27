use anyhow::Result;
use colored::*;

/// Check for missing dependencies before running a command
pub fn audit() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut issues = Vec::new();

    // Check Rust
    if cwd.join("Cargo.toml").exists() {
        if !cwd.join("Cargo.lock").exists() {
            issues.push(("Rust", "Cargo.lock missing — run `cargo build` or `cargo generate-lockfile`"));
        }
        if !cwd.join("target").exists() {
            issues.push(("Rust", "No target/ directory — dependencies not built yet"));
        }
    }

    // Check Node.js
    if cwd.join("package.json").exists() {
        if !cwd.join("node_modules").exists() {
            issues.push(("Node.js", "node_modules/ missing — run `npm install`"));
        }
        if !cwd.join("package-lock.json").exists() && !cwd.join("yarn.lock").exists() {
            issues.push(("Node.js", "No lock file — run `npm install` to create package-lock.json"));
        }
    }

    // Check Python
    if cwd.join("requirements.txt").exists() {
        let venv_exists = cwd.join("venv").exists() || cwd.join(".venv").exists();
        if !venv_exists {
            issues.push(("Python", "No virtual environment — run `python -m venv venv`"));
        }
    }

    // Check Go
    if cwd.join("go.mod").exists() {
        if !cwd.join("go.sum").exists() {
            issues.push(("Go", "go.sum missing — run `go mod tidy`"));
        }
    }

    // Check Docker
    if cwd.join("Dockerfile").exists() {
        if !cwd.join(".dockerignore").exists() {
            issues.push(("Docker", ".dockerignore missing — create one to optimize builds"));
        }
    }

    if issues.is_empty() {
        println!("{} {}", "veil".purple().bold(), "all dependencies ready ✓".green());
        return Ok(());
    }

    println!("{} {} dependency issues found:\n", "veil".purple().bold(), issues.len().to_string().yellow());

    for (lang, issue) in issues {
        println!(
            "  {} {} {}",
            format!("[{}]", lang).cyan(),
            "→".dimmed(),
            issue.yellow()
        );
    }

    println!();
    Ok(())
}
