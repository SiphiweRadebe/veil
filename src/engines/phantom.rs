use anyhow::Result;
use colored::*;

pub fn preview(cmd: &str) -> Result<()> {
    println!("{} {}\n", "Previewing:".purple().bold(), cmd.white().bold());
    let dangers = [
        ("rm -rf",    "permanently deletes files recursively"),
        ("rm -f",     "force-deletes without confirmation"),
        ("dd ",       "writes directly to disk"),
        ("chmod 777", "makes files world-writable"),
        ("sudo",      "runs with root privileges"),
        ("format",    "may format a drive"),
        ("del /f",    "force-deletes on Windows"),
        ("rd /s",     "deletes directory tree on Windows"),
    ];
    let mut found = false;
    for (pattern, explanation) in &dangers {
        if cmd.contains(pattern) {
            println!("  {} {} — {}", "!".red().bold(), pattern.yellow(), explanation.dimmed());
            found = true;
        }
    }
    if !found {
        println!("  {} No dangerous patterns detected.", "✓".green());
    }
    println!();
    println!("{}", "Full sandbox preview coming in v0.3.".dimmed());
    Ok(())
}