mod engines;

use clap::{Parser, Subcommand};
use anyhow::Result;
use colored::*;

#[derive(Parser)]
#[command(name = "veil")]
#[command(about = "A thin, intelligent layer over your terminal")]
#[command(version = "0.1.2")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Reverse the last destructive command
    Undo,
    /// Preview what a command would do before running it
    Preview {
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    /// Explain in plain English what the last command did
    Why,
    /// Show a live briefing of your current project
    Status,
    /// Search your personal command knowledge base
    Find {
        query: String,
    },
    /// Roll your terminal session back N minutes
    Back {
        minutes: u64,
    },
    /// Manage directory bookmarks
    Bookmark {
        #[command(subcommand)]
        action: BookmarkCommands,
    },
    /// Jump to a bookmarked directory
    Go {
        /// Name of the bookmark
        name: String,
    },
    /// Audit dependencies and environment
    Audit,
    /// Manage session recording and replay
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// Track and compare environment variables
    Env {
        #[command(subcommand)]
        action: EnvCommands,
    },
    /// Manage and suggest aliases
    Alias {
        #[command(subcommand)]
        action: AliasCommands,
    },
    #[command(hide = true)]
    Record {
        command: String,
        exit_code: i32,
        directory: String,
    },
    #[command(hide = true)]
    Snapshot {
        command: String,
        directory: String,
    },
}

#[derive(Subcommand)]
enum BookmarkCommands {
    /// Save current directory as a bookmark
    Add {
        /// Name for this bookmark
        name: String,
    },
    /// Remove a bookmark
    Remove {
        /// Name of the bookmark to remove
        name: String,
    },
    /// List all bookmarks
    List,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Replay recent commands from this session
    Replay {
        #[arg(default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum EnvCommands {
    /// Capture current environment as baseline
    Capture,
    /// Compare environment to baseline and report changes
    Diff,
}

#[derive(Subcommand)]
enum AliasCommands {
    /// Create a new alias
    Add {
        name: String,
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// List all aliases
    List,
    /// Suggest aliases based on command history
    Suggest,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Undo => {
            println!("{} {}", "veil".purple().bold(), "undo".white());
            engines::drift::undo()?;
        }
        Commands::Preview { cmd } => {
            let full_cmd = cmd.join(" ");
            println!("{} {} {}", "veil".purple().bold(), "preview".white(), full_cmd.dimmed());
            engines::phantom::preview(&full_cmd)?;
        }
        Commands::Why => {
            println!("{} {}", "veil".purple().bold(), "why".white());
            engines::trace::explain_last()?;
        }
        Commands::Status => {
            println!("{} {}", "veil".purple().bold(), "status".white());
            engines::context::status()?;
        }
        Commands::Find { query } => {
            println!("{} {} {}", "veil".purple().bold(), "find".white(), query.dimmed());
            engines::memoir::find(&query)?;
        }
        Commands::Back { minutes } => {
            println!("{} {} {}m", "veil".purple().bold(), "back".white(), minutes);
            engines::drift::go_back(minutes)?;
        }
        Commands::Bookmark { action } => {
            match action {
                BookmarkCommands::Add { name } => {
                    let dir = std::env::current_dir()?
                        .to_string_lossy()
                        .to_string();
                    engines::bookmarks::add(&name, &dir)?;
                }
                BookmarkCommands::Remove { name } => {
                    engines::bookmarks::remove(&name)?;
                }
                BookmarkCommands::List => {
                    engines::bookmarks::list()?;
                }
            }
        }
        Commands::Go { name } => {
            let path = engines::bookmarks::get(&name)?;
            // Print the path so the PowerShell hook can cd to it
            println!("VEIL_CD:{}", path);
        }
        Commands::Audit => {
            println!("{} {}", "veil".purple().bold(), "audit".white());
            engines::auditor::audit()?;
        }
        Commands::Session { action } => {
            match action {
                SessionCommands::Replay { limit } => {
                    engines::recorder::replay(limit)?;
                }
            }
        }
        Commands::Env { action } => {
            match action {
                EnvCommands::Capture => {
                    engines::envoy::capture()?;
                }
                EnvCommands::Diff => {
                    engines::envoy::diff()?;
                }
            }
        }
        Commands::Alias { action } => {
            match action {
                AliasCommands::Add { name, command } => {
                    let cmd_str = command.join(" ");
                    engines::sage::add_alias(&name, &cmd_str)?;
                }
                AliasCommands::List => {
                    engines::sage::list_aliases()?;
                }
                AliasCommands::Suggest => {
                    engines::sage::suggest()?;
                }
            }
        }
        Commands::Record { command, exit_code, directory } => {
            engines::memoir::record(&command, exit_code, &directory)?;
        }
        Commands::Snapshot { command, directory } => {
            engines::drift::snapshot(&command, &directory)?;
        }
    }

    Ok(())
}
