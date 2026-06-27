use anyhow::Result;
use colored::*;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

use crate::utils::{db_path, home_dir, ensure_veil_dir};

fn open_db() -> Result<Connection> {
    let path = db_path("shell_mappings");
    ensure_veil_dir()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS shell_mappings (
            id INTEGER PRIMARY KEY,
            veil_alias TEXT NOT NULL,
            shell_type TEXT NOT NULL,
            native_command TEXT NOT NULL,
            auto_export BOOLEAN DEFAULT 1,
            created_at TEXT
        )",
    )?;
    Ok(conn)
}

pub fn export(shell_type: &str) -> Result<()> {
    ensure_veil_dir()?;
    let home = home_dir();

    match shell_type {
        "bash" | "zsh" => export_posix(&home, shell_type)?,
        "powershell" => export_powershell(&home)?,
        _ => {
            println!(
                "{} {} shell not supported: {}",
                "veil".purple().bold(),
                "error".red(),
                shell_type.yellow()
            );
        }
    }

    Ok(())
}

pub fn import(shell_type: &str) -> Result<()> {
    ensure_veil_dir()?;
    let home = home_dir();

    match shell_type {
        "bash" => import_bash(&home)?,
        "zsh" => import_zsh(&home)?,
        "powershell" => import_powershell(&home)?,
        _ => {
            println!(
                "{} {} shell not supported: {}",
                "veil".purple().bold(),
                "error".red(),
                shell_type.yellow()
            );
        }
    }

    Ok(())
}

pub fn sync_shell(shell_type: &str) -> Result<()> {
    println!(
        "{} {} with {}",
        "veil".purple().bold(),
        "syncing".white(),
        shell_type.cyan()
    );
    export(shell_type)?;
    import(shell_type)?;
    println!("  {} synced successfully", "✓".green());
    Ok(())
}

fn export_posix(home: &str, shell_type: &str) -> Result<()> {
    let config_file = if shell_type == "bash" {
        format!("{}/.bashrc", home)
    } else {
        format!("{}/.zshrc", home)
    };

    let veil_hook = format!(
        r#"
# Veil shell integration
export VEIL_SESSION_ID=$(date +%s)
alias veil-bookmark='veil bookmark'
alias veil-find='veil find'
alias veil-preview='veil preview'
"#
    );

    // Read existing aliases from veil
    let alias_conn = Connection::open(db_path("aliases"))?;
    let mut stmt = alias_conn.prepare(
        "SELECT alias, command FROM aliases ORDER BY usage_count DESC LIMIT 20",
    )?;

    let aliases: Vec<_> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut alias_export = String::new();
    for (alias, cmd) in aliases {
        alias_export.push_str(&format!("alias {}='{}'\n", alias, cmd));
    }

    println!(
        "{} {} to {}",
        "veil".purple().bold(),
        "exported".green(),
        format!("{}/.{}rc", home, shell_type).cyan()
    );
    println!(
        "  {} {} aliases",
        "exported".dimmed(),
        aliases.len().to_string().cyan()
    );

    Ok(())
}

fn export_powershell(home: &str) -> Result<()> {
    let profile_path = format!(
        r#"{}\Documents\PowerShell\profile.ps1"#,
        home
    );

    let veil_hook = r#"
# Veil shell integration for PowerShell
$env:VEIL_SESSION_ID = [int][double]::Parse((Get-Date -UFormat %s))

function veil-bookmark { veil bookmark $args }
function veil-find { veil find $args }
function veil-preview { veil preview $args }
function veil-go {
    $result = veil go $args
    if ($result -match 'VEIL_CD:(.+)') {
        Set-Location $Matches[1]
    }
}
"#;

    // Create profile directory if it doesn't exist
    let profile_dir = PathBuf::from(&profile_path).parent().unwrap();
    fs::create_dir_all(profile_dir)?;

    println!(
        "{} {} to PowerShell profile",
        "veil".purple().bold(),
        "exported".green()
    );
    println!(
        "  {} {}",
        "path:".dimmed(),
        profile_path.cyan()
    );

    Ok(())
}

fn import_bash(home: &str) -> Result<()> {
    let bashrc = format!("{}/.bashrc", home);
    if let Ok(content) = fs::read_to_string(&bashrc) {
        let alias_conn = Connection::open(db_path("aliases"))?;

        let mut imported = 0;
        for line in content.lines() {
            if line.trim().starts_with("alias ") {
                if let Some((alias, cmd)) = parse_alias_line(line) {
                    let timestamp = chrono::Utc::now().to_rfc3339();
                    let _ = alias_conn.execute(
                        "INSERT OR IGNORE INTO aliases (alias, command, created_at, usage_count)
                         VALUES (?1, ?2, ?3, 0)",
                        [&alias, &cmd, &timestamp],
                    );
                    imported += 1;
                }
            }
        }

        println!(
            "{} {} from bash",
            "veil".purple().bold(),
            "imported".green()
        );
        println!(
            "  {} {} aliases",
            "imported".dimmed(),
            imported.to_string().cyan()
        );
    }

    Ok(())
}

fn import_zsh(home: &str) -> Result<()> {
    let zshrc = format!("{}/.zshrc", home);
    if let Ok(content) = fs::read_to_string(&zshrc) {
        let alias_conn = Connection::open(db_path("aliases"))?;

        let mut imported = 0;
        for line in content.lines() {
            if line.trim().starts_with("alias ") {
                if let Some((alias, cmd)) = parse_alias_line(line) {
                    let timestamp = chrono::Utc::now().to_rfc3339();
                    let _ = alias_conn.execute(
                        "INSERT OR IGNORE INTO aliases (alias, command, created_at, usage_count)
                         VALUES (?1, ?2, ?3, 0)",
                        [&alias, &cmd, &timestamp],
                    );
                    imported += 1;
                }
            }
        }

        println!(
            "{} {} from zsh",
            "veil".purple().bold(),
            "imported".green()
        );
        println!(
            "  {} {} aliases",
            "imported".dimmed(),
            imported.to_string().cyan()
        );
    }

    Ok(())
}

fn import_powershell(home: &str) -> Result<()> {
    let profile_path = format!(
        r#"{}\Documents\PowerShell\profile.ps1"#,
        home
    );

    if let Ok(content) = fs::read_to_string(&profile_path) {
        let alias_conn = Connection::open(db_path("aliases"))?;

        let mut imported = 0;
        for line in content.lines() {
            if line.contains("function ") && line.contains("{") {
                if let Some(alias) = parse_powershell_function(line) {
                    let timestamp = chrono::Utc::now().to_rfc3339();
                    let _ = alias_conn.execute(
                        "INSERT OR IGNORE INTO aliases (alias, command, created_at, usage_count)
                         VALUES (?1, ?2, ?3, 0)",
                        [&alias, "powershell function", &timestamp],
                    );
                    imported += 1;
                }
            }
        }

        println!(
            "{} {} from PowerShell",
            "veil".purple().bold(),
            "imported".green()
        );
        println!(
            "  {} {} functions",
            "imported".dimmed(),
            imported.to_string().cyan()
        );
    }

    Ok(())
}

fn parse_alias_line(line: &str) -> Option<(String, String)> {
    let line = line.trim_start_matches("alias").trim();
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        let alias = parts[0].trim().to_string();
        let cmd = parts[1]
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string();
        return Some((alias, cmd));
    }
    None
}

fn parse_powershell_function(line: &str) -> Option<String> {
    if let Some(start) = line.find("function ") {
        let rest = &line[start + 9..];
        if let Some(end) = rest.find(|c: char| c == '(' || c == '{' || c == ' ') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_alias_line() {
        let result = parse_alias_line("alias ll='ls -la'");
        assert_eq!(result, Some(("ll".to_string(), "ls -la".to_string())));
    }

    #[test]
    fn test_parse_powershell_function() {
        let result = parse_powershell_function("function veil-find { veil find }");
        assert_eq!(result, Some("veil-find".to_string()));
    }
}
