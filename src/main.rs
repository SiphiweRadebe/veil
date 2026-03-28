mod engines;

use clap::{Parser, Subcommand};
use anyhow::Result;
use colored::*;

#[derive(Parser)]
#[command(name = "veil")]
#[command(about = "A thin, intelligent layer over your terminal")]
#[command(version = "0.1.0")]
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
    #[command(hide = true)]
    Record {
        command: String,
        exit_code: i32,
        directory: String,
    },
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
        Commands::Record { command, exit_code, directory } => {
            engines::memoir::record(&command, exit_code, &directory)?;
        }
    }

    Ok(())
}