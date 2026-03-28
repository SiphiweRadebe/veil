use anyhow::Result;
use colored::*;

pub fn explain_last() -> Result<()> {
    println!("{}\n", "What did that command do?".purple().bold());
    println!("  {}", "Deep syscall tracing coming in v0.4.".dimmed());
    println!("  {}", "Try `veil find <query>` to search your history instead.".dimmed());
    Ok(())
}