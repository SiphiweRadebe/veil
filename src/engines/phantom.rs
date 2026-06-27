use anyhow::Result;
use colored::*;

use crate::engines::patterns::command_danger::{assess_command_risk, format_risk, RiskLevel};

pub fn preview(cmd: &str) -> Result<()> {
    println!("{} {}\n", "Preview:".purple().bold(), cmd.white().bold());

    let (risk_level, reason) = assess_command_risk(cmd);
    let risk_label = format_risk(&risk_level);

    println!("  {} {}", "risk:".dimmed(), risk_label);
    println!("  {} {}", "reason:".dimmed(), reason.yellow());
    println!();

    let dangers = detect_dangers(cmd);
    if !dangers.is_empty() {
        println!("  {}", "Specific concerns:".red().bold());
        for (pattern, explanation) in &dangers {
            println!("    {} {} — {}", "!".red().bold(), pattern.yellow(), explanation.dimmed());
        }
        println!();
    }

    let targets = predict_file_targets(cmd);
    if !targets.is_empty() {
        println!("  {}", "File targets detected:".dimmed());
        for target in &targets {
            let exists = std::path::Path::new(target).exists();
            let marker = if exists { "●".yellow() } else { "○".dimmed() };
            let label = if exists { "(exists)".yellow() } else { "(does not exist)".dimmed() };
            println!("    {} {} {}", marker, target.white(), label);
        }
        println!();
    }

    match risk_level {
        RiskLevel::High => {
            println!("  {}", "Recommendation: do not run without reviewing.".red().bold());
            println!("    → Run safely in isolation: `veil sandbox \"{}\"`", cmd);
        }
        RiskLevel::Medium => {
            println!(
                "  {} Review the output before proceeding.",
                "Recommendation:".yellow().bold()
            );
            println!("    → Optionally isolate: `veil sandbox \"{}\"`", cmd);
        }
        RiskLevel::Safe => {
            println!("  {} Safe to run.", "✓".green().bold());
        }
    }

    println!();
    Ok(())
}

fn predict_file_targets(cmd: &str) -> Vec<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let mut targets = Vec::new();
    let mut skip_next = false;

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if skip_next {
            skip_next = false;
            continue;
        }
        if matches!(*part, "-m" | "-t" | "-o" | "--tag" | "--message" | "--output" | "-f" | "--file") {
            skip_next = true;
            continue;
        }
        if part.starts_with('-') {
            continue;
        }
        if part.contains('/') || part.contains('\\') || part.starts_with('.') || *part == ".." {
            targets.push(part.to_string());
        }
    }
    targets
}

fn detect_dangers(cmd: &str) -> Vec<(String, &'static str)> {
    let checks: &[(&str, &str)] = &[
        ("rm -rf", "permanently deletes files recursively — no recycle bin"),
        ("rm -f", "force-deletes without confirmation prompt"),
        ("dd ", "writes directly to disk, can overwrite entire partitions"),
        ("chmod 777", "makes files world-writable — security risk"),
        ("sudo", "runs with elevated root privileges"),
        ("del /f", "force-deletes files on Windows"),
        ("rd /s", "removes entire directory tree on Windows"),
        ("DROP TABLE", "destroys a database table permanently"),
        ("DROP DATABASE", "destroys an entire database permanently"),
        ("git push --force", "overwrites remote branch history — affects all teammates"),
        ("git push -f", "overwrites remote branch history — affects all teammates"),
        ("git reset --hard", "discards ALL uncommitted changes — irreversible"),
        ("> /dev/", "writes output directly to a device file"),
        ("mkfs", "formats a filesystem — destroys all data on the device"),
    ];

    let cmd_lower = cmd.to_lowercase();
    checks
        .iter()
        .filter(|(pattern, _)| cmd_lower.contains(&pattern.to_lowercase()))
        .map(|(pattern, explanation)| (pattern.to_string(), *explanation))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_safe_command() {
        let result = preview("ls -la");
        assert!(result.is_ok());
    }

    #[test]
    fn test_preview_dangerous_command() {
        let result = preview("rm -rf /tmp/test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_predict_file_targets() {
        let targets = predict_file_targets("cp ./src/main.rs ./backup/main.rs");
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_detect_dangers() {
        let dangers = detect_dangers("rm -rf /");
        assert!(!dangers.is_empty());
        assert!(dangers[0].0.contains("rm -rf"));
    }
}
