use anyhow::Result;
use colored::*;
use std::process::Command;

pub fn audit() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut issues: Vec<(String, String)> = Vec::new();
    let mut checked: Vec<String> = Vec::new();

    // ── Rust ──────────────────────────────────────────────────────────────────
    if cwd.join("Cargo.toml").exists() {
        checked.push("Rust".to_string());

        if !cwd.join("Cargo.lock").exists() {
            issues.push(("Rust".to_string(), "Cargo.lock missing — run `cargo generate-lockfile`".to_string()));
        } else {
            // Run cargo check for actual type/compile errors
            let result = Command::new("cargo")
                .arg("check")
                .arg("--message-format=short")
                .current_dir(&cwd)
                .output();

            match result {
                Ok(output) if !output.status.success() => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error_count = stderr.lines().filter(|l| l.contains("error[")).count();
                    if error_count > 0 {
                        issues.push(("Rust".to_string(), format!("`cargo check` found {} error(s) — run `cargo check` for details", error_count)));
                    } else {
                        // Show first warning if no errors
                        let first_warn = stderr.lines().find(|l| l.contains("warning:"));
                        if let Some(w) = first_warn {
                            let short = w.trim().chars().take(80).collect::<String>();
                            issues.push(("Rust".to_string(), format!("warnings detected: {}", short)));
                        }
                    }
                }
                Err(_) => {
                    // cargo not found — just check file presence
                    if !cwd.join("target").exists() {
                        issues.push(("Rust".to_string(), "target/ not built — run `cargo build`".to_string()));
                    }
                }
                _ => {} // clean
            }
        }
    }

    // ── Node.js ───────────────────────────────────────────────────────────────
    if cwd.join("package.json").exists() {
        checked.push("Node.js".to_string());

        if !cwd.join("node_modules").exists() {
            issues.push(("Node.js".to_string(), "node_modules/ missing — run `npm install`".to_string()));
        } else {
            let result = Command::new("npm")
                .args(["list", "--depth=0", "--json"])
                .current_dir(&cwd)
                .output();

            match result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // npm list --json exits non-zero if there are peer/unmet deps
                    if !output.status.success() {
                        // Count "UNMET DEPENDENCY" lines in the plain output
                        let plain = Command::new("npm")
                            .args(["list", "--depth=0"])
                            .current_dir(&cwd)
                            .output()
                            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                            .unwrap_or_default();
                        let unmet = plain.lines().filter(|l| l.contains("UNMET")).count();
                        if unmet > 0 {
                            issues.push(("Node.js".to_string(), format!("{} unmet dependencies — run `npm install`", unmet)));
                        }
                    }
                    let _ = stdout;
                }
                Err(_) => {
                    if !cwd.join("package-lock.json").exists() && !cwd.join("yarn.lock").exists() {
                        issues.push(("Node.js".to_string(), "no lock file — run `npm install`".to_string()));
                    }
                }
            }
        }
    }

    // ── Python ────────────────────────────────────────────────────────────────
    if cwd.join("requirements.txt").exists() {
        checked.push("Python".to_string());

        let venv_exists = cwd.join("venv").exists() || cwd.join(".venv").exists();
        if !venv_exists {
            issues.push(("Python".to_string(), "no virtual environment — run `python -m venv venv`".to_string()));
        } else {
            // Try pip check
            let pip_bin = if cwd.join("venv/bin/pip").exists() {
                "venv/bin/pip"
            } else if cwd.join(".venv/bin/pip").exists() {
                ".venv/bin/pip"
            } else {
                "pip"
            };

            let result = Command::new(pip_bin)
                .arg("check")
                .current_dir(&cwd)
                .output();

            match result {
                Ok(output) if !output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let conflict_count = stdout.lines().count();
                    issues.push(("Python".to_string(), format!("{} dependency conflict(s) — run `pip check` for details", conflict_count)));
                }
                _ => {}
            }
        }
    }

    // ── Go ────────────────────────────────────────────────────────────────────
    if cwd.join("go.mod").exists() {
        checked.push("Go".to_string());

        if !cwd.join("go.sum").exists() {
            issues.push(("Go".to_string(), "go.sum missing — run `go mod tidy`".to_string()));
        } else {
            let result = Command::new("go")
                .args(["mod", "verify"])
                .current_dir(&cwd)
                .output();

            match result {
                Ok(output) if !output.status.success() => {
                    issues.push(("Go".to_string(), "go mod verify failed — dependencies may be corrupt, run `go mod tidy`".to_string()));
                }
                _ => {}
            }
        }
    }

    // ── Docker ────────────────────────────────────────────────────────────────
    if cwd.join("Dockerfile").exists() {
        checked.push("Docker".to_string());

        if !cwd.join(".dockerignore").exists() {
            issues.push(("Docker".to_string(), ".dockerignore missing — builds may include unnecessary files".to_string()));
        }

        // Check if docker daemon is reachable
        let result = Command::new("docker").arg("info").output();
        match result {
            Ok(output) if !output.status.success() => {
                issues.push(("Docker".to_string(), "Docker daemon not running — start Docker Desktop or `sudo systemctl start docker`".to_string()));
            }
            _ => {}
        }
    }

    // ── Output ────────────────────────────────────────────────────────────────
    if checked.is_empty() {
        println!(
            "{} {} no recognized project type in this directory",
            "veil".purple().bold(),
            "audit skipped —".yellow()
        );
        println!("  {}", "Supported: Rust (Cargo.toml), Node (package.json), Python (requirements.txt), Go (go.mod), Docker (Dockerfile)".dimmed());
        return Ok(());
    }

    println!(
        "{} {} ({})\n",
        "veil".purple().bold(),
        "audit".white(),
        checked.join(", ").cyan()
    );

    if issues.is_empty() {
        println!("  {} all dependencies verified ✓", "".green());
        return Ok(());
    }

    println!(
        "  {} {} issue(s) found:\n",
        "⚠".yellow(),
        issues.len().to_string().yellow()
    );

    for (lang, issue) in &issues {
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
