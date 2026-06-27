use anyhow::Result;
use std::path::PathBuf;
use colored::*;

pub fn home_dir() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string())
}

pub fn veil_dir() -> PathBuf {
    let home = home_dir();
    PathBuf::from(format!("{}/.veil", home))
}

pub fn ensure_veil_dir() -> Result<PathBuf> {
    let dir = veil_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn db_path(name: &str) -> String {
    let dir = veil_dir();
    dir.join(format!("{}.db", name))
        .to_string_lossy()
        .to_string()
}

pub fn data_path(filename: &str) -> String {
    let dir = veil_dir();
    dir.join(filename)
        .to_string_lossy()
        .to_string()
}

pub fn format_success(label: &str) -> String {
    format!("{} {} {}", "veil".purple().bold(), label.white(), "✓".green())
}

pub fn format_warning(label: &str, msg: &str) -> String {
    format!("{} {} {}", label.yellow(), "→".dimmed(), msg.yellow())
}

pub fn format_error(label: &str, msg: &str) -> String {
    format!("{} {} {}", label.red(), "→".dimmed(), msg.red())
}

pub fn format_info(label: &str, msg: &str) -> String {
    format!("{} {} {}", "veil".purple().bold(), label.white(), msg.dimmed())
}

/// Returns a shell Command that works on both Windows (cmd /C) and Unix (sh -c).
pub fn shell_exec(cmd: &str) -> std::process::Command {
    if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", cmd]);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.args(["-c", cmd]);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_dir() {
        let home = home_dir();
        assert!(!home.is_empty());
    }

    #[test]
    fn test_veil_dir() {
        let dir = veil_dir();
        assert!(dir.to_string_lossy().contains(".veil"));
    }

    #[test]
    fn test_db_path() {
        let path = db_path("test");
        assert!(path.contains("test.db"));
    }
}
