mod engines;
mod utils;

use clap::{Parser, Subcommand};
use anyhow::Result;
use colored::*;

#[derive(Parser)]
#[command(name = "veil")]
#[command(about = "A thin, intelligent layer over your terminal")]
#[command(version = "2.0.0")]
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
        name: String,
    },
    /// Rewind terminal to a previous state
    Rewind {
        #[arg(default_value = "5")]
        minutes: u64,
    },
    /// Show timeline of snapshots
    Timeline {
        #[arg(default_value = "10")]
        limit: usize,
    },
    /// Replay a snapshot from a specific time
    Play {
        time: String,
    },
    /// Run command in isolated sandbox
    Sandbox {
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    /// Predict command side effects before running
    Whatif {
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    /// Find similar commands from history
    Related {
        query: String,
    },
    /// Manage recurring workflows
    Workflow {
        #[command(subcommand)]
        action: WorkflowCommands,
    },
    /// Suggest next command based on history
    Next,
    /// Analyze project health and dependencies
    Analyze,
    /// Show dependency graph
    Deps {
        #[arg(long, default_value = "visual")]
        format: String,
    },
    /// Analyze impact of file changes
    Impact {
        file: String,
    },
    /// Audit dependencies and environment
    Audit,
    /// Manage session recording and replay
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// Remote host execution and management
    Remote {
        #[command(subcommand)]
        action: RemoteCommands,
    },
    /// Team collaboration and shared configs
    Team {
        #[command(subcommand)]
        action: TeamCommands,
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
    Add { name: String },
    /// Remove a bookmark
    Remove { name: String },
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
enum WorkflowCommands {
    /// List all saved workflows
    List,
    /// Save current command sequence as workflow
    Save { name: String },
}

#[derive(Subcommand)]
enum RemoteCommands {
    /// Add a remote host
    Add {
        name: String,
        host: String,
        user: String,
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        tags: Option<String>,
    },
    /// List all remote hosts
    List,
    /// Execute command on remote host
    Ssh {
        host: String,
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Share session replay
    Share { session_id: String },
    /// Remove remote host
    Remove { name: String },
}

#[derive(Subcommand)]
enum TeamCommands {
    /// Setup team configuration
    Setup {
        name: String,
        remote_type: String,
        url: String,
    },
    /// List team configurations
    List,
    /// Share a bookmark or workflow with team
    Share {
        #[command(subcommand)]
        action: TeamShareCommands,
    },
    /// Pull updates from team
    Pull,
}

#[derive(Subcommand)]
enum TeamShareCommands {
    /// Share a bookmark
    Bookmark {
        name: String,
        #[arg(default_value = "")]
        description: String,
    },
    /// Share a workflow
    Workflow {
        name: String,
        #[arg(default_value = "")]
        description: String,
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
        Commands::Bookmark { action } => {
            match action {
                BookmarkCommands::Add { name } => {
                    let dir = std::env::current_dir()?.to_string_lossy().to_string();
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
            println!("VEIL_CD:{}", path);
        }
        Commands::Rewind { minutes } => {
            engines::temporal::rewind(minutes)?;
        }
        Commands::Timeline { limit } => {
            engines::temporal::timeline(limit)?;
        }
        Commands::Play { time } => {
            engines::temporal::play(&time)?;
        }
        Commands::Sandbox { cmd } => {
            let full_cmd = cmd.join(" ");
            println!("{} {} {}", "veil".purple().bold(), "sandbox".white(), full_cmd.dimmed());
            engines::sandbox::sandbox(&full_cmd)?;
        }
        Commands::Whatif { cmd } => {
            let full_cmd = cmd.join(" ");
            engines::sandbox::whatif(&full_cmd)?;
        }
        Commands::Related { query } => {
            engines::context_suggest::related(&query)?;
        }
        Commands::Workflow { action } => {
            match action {
                WorkflowCommands::List => {
                    engines::context_suggest::workflow_list()?;
                }
                WorkflowCommands::Save { name } => {
                    engines::context_suggest::workflow_save(&name)?;
                }
            }
        }
        Commands::Next => {
            engines::context_suggest::next()?;
        }
        Commands::Analyze => {
            println!("{} {}", "veil".purple().bold(), "analyze".white());
            engines::analyzer::analyze()?;
        }
        Commands::Deps { format } => {
            if format == "json" {
                engines::analyzer::deps_json()?;
            } else {
                engines::analyzer::deps_visual()?;
            }
        }
        Commands::Impact { file } => {
            engines::analyzer::impact(&file)?;
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
        Commands::Remote { action } => {
            match action {
                RemoteCommands::Add { name, host, user, key, tags } => {
                    engines::remote::add_host(&name, &host, &user, key.as_deref(), tags.as_deref())?;
                }
                RemoteCommands::List => {
                    engines::remote::host_list()?;
                }
                RemoteCommands::Ssh { host, command } => {
                    let cmd_str = command.join(" ");
                    println!("{} {} ssh {}", "veil".purple().bold(), "remote".white(), host.cyan());
                    engines::remote::ssh(&host, &cmd_str)?;
                }
                RemoteCommands::Share { session_id } => {
                    engines::remote::replay_share(&session_id)?;
                }
                RemoteCommands::Remove { name } => {
                    engines::remote::host_remove(&name)?;
                }
            }
        }
        Commands::Team { action } => {
            match action {
                TeamCommands::Setup { name, remote_type, url } => {
                    engines::team::setup_team(&name, &remote_type, &url)?;
                }
                TeamCommands::List => {
                    engines::team::team_list()?;
                }
                TeamCommands::Share { action } => {
                    match action {
                        TeamShareCommands::Bookmark { name, description } => {
                            engines::team::share_bookmark(&name, &description)?;
                        }
                        TeamShareCommands::Workflow { name, description } => {
                            engines::team::share_workflow(&name, &description)?;
                        }
                    }
                }
                TeamCommands::Pull => {
                    engines::team::team_pull()?;
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
