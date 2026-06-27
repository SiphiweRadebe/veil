use anyhow::Result;
use colored::*;
use rusqlite::Connection;

use crate::utils::{db_path, ensure_veil_dir};

pub fn explain_last() -> Result<()> {
    ensure_veil_dir()?;
    let memoir_path = db_path("memoir");
    let conn = match Connection::open(&memoir_path) {
        Ok(c) => c,
        Err(_) => {
            println!("{} {}", "veil".purple().bold(), "no command history yet.".dimmed());
            return Ok(());
        }
    };

    let result = conn.query_row(
        "SELECT command, directory, exit_code, timestamp FROM commands ORDER BY timestamp DESC LIMIT 1",
        [],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, String>(3)?,
        )),
    );

    let (command, directory, exit_code, timestamp) = match result {
        Ok(r) => r,
        Err(_) => {
            println!("{} {}", "veil".purple().bold(), "no commands recorded yet.".dimmed());
            println!("  {}", "run a command first, then try `veil why` again.".dimmed());
            return Ok(());
        }
    };

    let short_time = if timestamp.len() >= 16 { &timestamp[..16] } else { &timestamp };
    let short_time = short_time.replace('T', " ");

    println!("{} {}\n", "Last command:".purple().bold(), command.white().bold());
    println!("  {} {}  {}", "in".dimmed(), directory.cyan(), short_time.dimmed());
    if exit_code == 0 {
        println!("  {} {}", "exit code:".dimmed(), "0 (success)".green());
    } else {
        println!("  {} {}", "exit code:".dimmed(), format!("{} (failed)", exit_code).red());
    }
    println!();

    let explanation = explain_command(&command);
    println!("{}", "What it did:".purple().bold());
    for line in &explanation {
        println!("  {} {}", "→".dimmed(), line.white());
    }

    if exit_code != 0 {
        println!();
        let base = command.split_whitespace().next().unwrap_or("");
        println!(
            "  {} try `{} --help` or search history with `veil find {}`",
            "hint:".yellow(),
            base,
            base
        );
    }

    println!();
    Ok(())
}

fn explain_command(cmd: &str) -> Vec<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let base = parts.first().unwrap_or(&"");
    let args = if parts.len() > 1 { &parts[1..] } else { &[] as &[&str] };

    match *base {
        "git" => explain_git(args),
        "cargo" => explain_cargo(args),
        "npm" | "yarn" | "pnpm" => explain_node(base, args),
        "rm" => explain_rm(args),
        "cp" => explain_cp(args),
        "mv" => explain_mv(args),
        "ls" | "dir" => vec!["Listed directory contents".to_string()],
        "cd" => {
            let dir = args.first().unwrap_or(&"~");
            vec![format!("Changed working directory to {}", dir)]
        }
        "mkdir" => {
            let dirs: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
            vec![format!(
                "Created director{}: {}",
                if dirs.len() > 1 { "ies" } else { "y" },
                dirs.join(", ")
            )]
        }
        "cat" | "type" => {
            let files: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
            vec![format!("Printed content of: {}", files.join(", "))]
        }
        "echo" => vec![format!("Printed text to stdout: {}", args.join(" "))],
        "grep" | "findstr" => {
            let pattern = args.first().unwrap_or(&"<pattern>");
            vec![
                format!("Searched for pattern: {}", pattern),
                "Printed lines matching the pattern".to_string(),
            ]
        }
        "find" => vec![
            "Recursively searched for files matching criteria".to_string(),
            format!("With args: {}", args.join(" ")),
        ],
        "curl" | "wget" => {
            let url = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<url>");
            vec![format!("Made HTTP request to: {}", url)]
        }
        "docker" => explain_docker(args),
        "python" | "python3" | "py" => {
            let script = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<script>");
            vec![format!("Ran Python: {}", script)]
        }
        "node" => {
            let script = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<script>");
            vec![format!("Ran Node.js: {}", script)]
        }
        "make" => {
            let target = args.first().unwrap_or(&"default");
            vec![format!("Ran make target: {}", target)]
        }
        "ssh" => {
            let host = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<host>");
            vec![format!("Connected to remote host: {}", host)]
        }
        "sudo" => {
            let rest = args.join(" ");
            let mut v = vec!["Ran with elevated (root) privileges:".to_string()];
            v.extend(explain_command(&rest));
            v
        }
        "touch" => {
            let files: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
            vec![format!("Created empty file(s) or updated timestamps: {}", files.join(", "))]
        }
        "chmod" => {
            let perms = args.first().unwrap_or(&"<perms>");
            let files: Vec<_> = args.iter().skip(1).filter(|a| !a.starts_with('-')).collect();
            vec![format!("Changed permissions of {} to {}", files.join(", "), perms)]
        }
        "chown" => vec!["Changed file ownership".to_string()],
        "kill" | "pkill" | "killall" => {
            let target = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<process>");
            vec![format!("Sent termination signal to process: {}", target)]
        }
        "ps" => vec!["Listed running processes".to_string()],
        "top" | "htop" => vec!["Opened interactive process monitor".to_string()],
        "ping" => {
            let host = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<host>");
            vec![format!("Tested network connectivity to: {}", host)]
        }
        _ => vec![
            format!("Ran: {}", cmd),
            format!("No built-in explanation for `{}` — try `{} --help`", base, base),
        ],
    }
}

fn explain_git(args: &[&str]) -> Vec<String> {
    let sub = args.first().unwrap_or(&"");
    match *sub {
        "add" => {
            let files: Vec<_> = args[1..].iter().filter(|a| !a.starts_with('-')).collect();
            let what = if files.iter().any(|f| **f == "." || **f == "-A" || **f == "--all") || files.is_empty() {
                "all changed files".to_string()
            } else {
                files.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
            };
            vec![format!("Staged {} for the next commit", what)]
        }
        "commit" => {
            let msg_idx = args.iter().position(|a| *a == "-m" || *a == "--message");
            let msg = msg_idx
                .and_then(|i| args.get(i + 1))
                .map(|m| m.trim_matches('"').trim_matches('\'').to_string())
                .unwrap_or_else(|| "<no -m flag found>".to_string());
            vec![
                format!("Created a new commit: \"{}\"", msg),
                "Saved staged changes permanently to repository history".to_string(),
            ]
        }
        "push" => {
            let remote = args.get(1).unwrap_or(&"origin");
            let branch = args.get(2).unwrap_or(&"current branch");
            let force = args.contains(&"--force") || args.contains(&"-f");
            let mut lines = vec![format!("Pushed local commits to {}/{}", remote, branch)];
            if force {
                lines.push("Force push: overwrote remote history".to_string());
            }
            lines
        }
        "pull" => {
            let remote = args.get(1).unwrap_or(&"origin");
            vec![
                format!("Fetched and merged latest changes from {}", remote),
                "Updated local branch with remote commits".to_string(),
            ]
        }
        "clone" => {
            let repo = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<repo>");
            vec![
                format!("Cloned repository: {}", repo),
                "Downloaded full history and all files locally".to_string(),
            ]
        }
        "status" => vec!["Checked working tree: showed staged, unstaged, and untracked files".to_string()],
        "log" => vec!["Displayed commit history for the current branch".to_string()],
        "diff" => {
            if args.contains(&"--cached") || args.contains(&"--staged") {
                vec!["Showed diff of staged changes (vs last commit)".to_string()]
            } else {
                vec!["Showed diff of unstaged changes in working tree".to_string()]
            }
        }
        "checkout" | "switch" => {
            let target = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<branch>");
            if args.contains(&"-b") {
                vec![format!("Created and switched to new branch: {}", target)]
            } else {
                vec![format!("Switched to branch: {}", target)]
            }
        }
        "branch" => {
            if args.iter().any(|a| *a == "-d" || *a == "-D") {
                let branch = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<branch>");
                vec![format!("Deleted branch: {}", branch)]
            } else {
                vec!["Listed or created branches".to_string()]
            }
        }
        "merge" => {
            let branch = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<branch>");
            vec![
                format!("Merged branch {} into current branch", branch),
                "Combined commit histories".to_string(),
            ]
        }
        "rebase" => {
            let onto = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<branch>");
            vec![format!("Replayed current branch commits on top of {}", onto)]
        }
        "stash" => {
            let sub2 = args.get(1).unwrap_or(&"push");
            vec![match *sub2 {
                "pop" | "apply" => "Restored stashed changes back to working tree".to_string(),
                "list" => "Listed all saved stashes".to_string(),
                "drop" => "Deleted a stash entry".to_string(),
                _ => "Saved uncommitted changes to a temporary stash".to_string(),
            }]
        }
        "reset" => {
            if args.contains(&"--hard") {
                vec![
                    "Discarded all uncommitted changes (PERMANENT)".to_string(),
                    "Reset working tree and index to last commit".to_string(),
                ]
            } else if args.contains(&"--soft") {
                vec!["Moved HEAD to specified commit, kept changes staged".to_string()]
            } else {
                vec!["Unstaged changes (files kept modified in working tree)".to_string()]
            }
        }
        "fetch" => vec!["Downloaded remote changes without merging or modifying working tree".to_string()],
        "tag" => {
            let tag = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<tag>");
            vec![format!("Created or listed tag: {}", tag)]
        }
        "remote" => vec!["Managed remote repository connections".to_string()],
        "init" => vec!["Initialized a new empty Git repository in current directory".to_string()],
        _ => vec![format!("Ran git {}", args.join(" "))],
    }
}

fn explain_cargo(args: &[&str]) -> Vec<String> {
    let sub = args.first().unwrap_or(&"");
    match *sub {
        "build" | "b" => {
            if args.contains(&"--release") {
                vec!["Compiled project in release mode (fully optimized, stripped debug info)".to_string()]
            } else {
                vec!["Compiled project in debug mode (fast to compile, includes debug symbols)".to_string()]
            }
        }
        "run" | "r" => {
            if args.contains(&"--release") {
                vec!["Compiled (release) and ran the project binary".to_string()]
            } else {
                vec!["Compiled (debug) and ran the project binary".to_string()]
            }
        }
        "test" | "t" => vec![
            "Compiled and ran all unit and integration tests".to_string(),
            "Reported pass/fail for each test function".to_string(),
        ],
        "check" | "c" => vec![
            "Type-checked the project without producing a binary".to_string(),
            "Faster than build — good for catching errors quickly".to_string(),
        ],
        "clean" => vec!["Deleted the target/ directory (removed all compiled artifacts)".to_string()],
        "add" => {
            let dep = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<dep>");
            vec![format!("Added dependency `{}` to Cargo.toml", dep)]
        }
        "remove" => {
            let dep = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<dep>");
            vec![format!("Removed dependency `{}` from Cargo.toml", dep)]
        }
        "install" => {
            let pkg = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<package>");
            vec![format!("Installed binary from crate `{}` to ~/.cargo/bin/", pkg)]
        }
        "publish" => vec!["Published this crate to crates.io (public registry)".to_string()],
        "fmt" => vec!["Formatted all Rust source files with rustfmt".to_string()],
        "clippy" => vec![
            "Ran linter on all Rust source files".to_string(),
            "Checked for common mistakes, style issues, and potential bugs".to_string(),
        ],
        "update" => vec!["Updated dependency versions in Cargo.lock to latest compatible versions".to_string()],
        "doc" => vec!["Generated HTML documentation for this crate".to_string()],
        _ => vec![format!("Ran cargo {}", args.join(" "))],
    }
}

fn explain_node(pm: &str, args: &[&str]) -> Vec<String> {
    let sub = args.first().unwrap_or(&"");
    match *sub {
        "install" | "i" | "ci" => vec![
            "Installed all dependencies listed in package.json".to_string(),
            "Created or updated node_modules/ directory".to_string(),
        ],
        "run" => {
            let script = args.get(1).unwrap_or(&"<script>");
            vec![format!("Ran script \"{}\" defined in package.json", script)]
        }
        "build" => vec!["Built project using the build script from package.json".to_string()],
        "test" => vec!["Ran test suite".to_string()],
        "start" => vec!["Started the application server (ran start script)".to_string()],
        "publish" => vec![format!("Published package to npm registry using {}", pm)],
        "add" | "install " => {
            let pkg = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<package>");
            vec![format!("Installed package: {}", pkg)]
        }
        "remove" | "uninstall" => {
            let pkg = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<package>");
            vec![format!("Removed package: {}", pkg)]
        }
        _ => vec![format!("Ran {} {}", pm, args.join(" "))],
    }
}

fn explain_rm(args: &[&str]) -> Vec<String> {
    let recursive = args.iter().any(|a| *a == "-r" || *a == "-R" || a.contains('r'));
    let force = args.iter().any(|a| a.contains('f'));
    let files: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let mut lines = Vec::new();
    if recursive && force {
        lines.push(format!(
            "Force-deleted recursively (permanent): {}",
            files.join(", ")
        ));
        lines.push("No recycle bin — files are gone unless veil has a snapshot".to_string());
    } else if recursive {
        lines.push(format!("Deleted directory tree: {}", files.join(", ")));
    } else {
        lines.push(format!("Deleted file(s): {}", files.join(", ")));
    }
    lines
}

fn explain_cp(args: &[&str]) -> Vec<String> {
    let files: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if files.len() >= 2 {
        vec![format!(
            "Copied {} → {}",
            files[..files.len() - 1].join(", "),
            files[files.len() - 1]
        )]
    } else {
        vec!["Copied files".to_string()]
    }
}

fn explain_mv(args: &[&str]) -> Vec<String> {
    let files: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if files.len() >= 2 {
        vec![format!(
            "Moved/renamed {} → {}",
            files[..files.len() - 1].join(", "),
            files[files.len() - 1]
        )]
    } else {
        vec!["Moved or renamed files".to_string()]
    }
}

fn explain_docker(args: &[&str]) -> Vec<String> {
    let sub = args.first().unwrap_or(&"");
    match *sub {
        "build" => {
            let tag = args
                .iter()
                .position(|a| *a == "-t" || *a == "--tag")
                .and_then(|i| args.get(i + 1))
                .unwrap_or(&"<image>");
            vec![format!("Built Docker image: {}", tag)]
        }
        "run" => {
            let image = args.iter().rev().find(|a| !a.starts_with('-')).unwrap_or(&"<image>");
            let detached = args.contains(&"-d");
            let mut lines = vec![format!("Started container from image: {}", image)];
            if detached {
                lines.push("Running in background (detached)".to_string());
            }
            lines
        }
        "ps" => vec![
            if args.contains(&"-a") || args.contains(&"--all") {
                "Listed all containers (including stopped)".to_string()
            } else {
                "Listed running containers".to_string()
            }
        ],
        "stop" => vec!["Stopped running container(s) gracefully".to_string()],
        "rm" => vec!["Removed stopped container(s)".to_string()],
        "rmi" => vec!["Removed Docker image(s) from local storage".to_string()],
        "pull" => {
            let image = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<image>");
            vec![format!("Downloaded Docker image from registry: {}", image)]
        }
        "push" => {
            let image = args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"<image>");
            vec![format!("Pushed image to registry: {}", image)]
        }
        "compose" | "stack" => {
            let sub2 = args.get(1).unwrap_or(&"");
            vec![match *sub2 {
                "up" => "Started all services defined in docker-compose.yml".to_string(),
                "down" => "Stopped and removed all compose services and networks".to_string(),
                "build" => "Built images for all compose services".to_string(),
                "logs" => "Showed logs from compose services".to_string(),
                _ => format!("Ran docker {} {}", sub, args[1..].join(" ")),
            }]
        }
        "exec" => vec!["Ran a command inside a running container".to_string()],
        "logs" => vec!["Showed logs from a container".to_string()],
        "images" => vec!["Listed all locally stored Docker images".to_string()],
        _ => vec![format!("Ran docker {}", args.join(" "))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_git_commit() {
        let result = explain_command("git commit -m \"fix bug\"");
        assert!(!result.is_empty());
        assert!(result[0].contains("commit") || result[0].contains("fix bug"));
    }

    #[test]
    fn test_explain_cargo_build() {
        let result = explain_command("cargo build --release");
        assert!(!result.is_empty());
        assert!(result[0].contains("release"));
    }

    #[test]
    fn test_explain_rm_rf() {
        let result = explain_command("rm -rf /tmp/test");
        assert!(result[0].contains("Force-deleted") || result[0].contains("deleted"));
    }

    #[test]
    fn test_explain_unknown() {
        let result = explain_command("somerandombinary --flag");
        assert!(!result.is_empty());
    }
}
