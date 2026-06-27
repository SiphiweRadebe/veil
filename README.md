<p align="center">
  <img src="assets/logo.svg" width="400" alt="veil logo"/>
</p>

<p align="center">
  <strong>A thin, intelligent layer over your terminal.</strong><br/>
  Undo mistakes. Predict risk. Learn your patterns. Run anywhere.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/built_with-Rust-orange?style=flat-square"/>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=flat-square"/>
  <img src="https://img.shields.io/badge/version-2.0.0-purple?style=flat-square"/>
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square"/>
  <a href="https://crates.io/crates/veil-shell"><img src="https://img.shields.io/crates/v/veil-shell?style=flat-square&color=purple"/></a>
</p>

---

## Why veil?

Your terminal doesn't protect you. It executes what you type, no questions asked. Veil adds an intelligent layer that watches, learns, and intervenes — things your shell simply doesn't do.

The key question when building veil: *does this already exist?* Aliases? Your shell has them. File watching? Use `watchexec`. Cron jobs? Use cron. Veil only ships what no other tool gives you.

**What veil actually does that nothing else does:**

- **Undo destructive commands** — real filesystem rollback, not just `Ctrl+Z`
- **Sandbox any command** — run it in an isolated copy of your project, nothing touches your real files
- **Predict side effects** — `veil whatif rm -rf dist` tells you *exactly* what would be destroyed before you run it
- **Travel back in time** — rewind your terminal to how it looked 20 minutes ago
- **Explain what just happened** — `veil why` translates the last command into plain English
- **Learn your patterns** — detect recurring command sequences and replay them as named workflows
- **Deep project intelligence** — dependency graphs, dead code detection, impact analysis
- **Remote execution** — run commands on registered SSH hosts, share session replays with your team

---

## Get veil

### Option 1: Download a binary (fastest, no Rust required)

Grab a pre-built binary from **[GitHub Releases](https://github.com/SiphiweRadebe/veil/releases)** — Windows `.exe`, Linux, and macOS binaries are built and published automatically on every release.

```powershell
# Windows — move to somewhere in your PATH
move veil-windows.exe C:\Users\<you>\.cargo\bin\veil.exe

# Linux
chmod +x veil-linux && mv veil-linux ~/.local/bin/veil

# macOS
chmod +x veil-macos && mv veil-macos /usr/local/bin/veil
```

### Option 2: Install via cargo

```bash
cargo install veil-shell
```

Requires Rust 1.70+ and a C linker (MSVC on Windows, `gcc` on Linux/macOS). If you're on Windows and don't have the MSVC linker, install [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/) with the "Desktop development with C++" workload.

### Option 3: Build from source

```bash
git clone https://github.com/SiphiweRadebe/veil.git
cd veil
cargo build --release
# Binary: target/release/veil  (or veil.exe on Windows)
```

### Shell hook (recommended)

Add veil's hook so it records your commands automatically:

**PowerShell** — add to your `$PROFILE`:
```powershell
. "C:\path\to\veil\hook.ps1"
```

**bash / zsh** — add to `.bashrc` / `.zshrc`:
```bash
source ~/path/to/veil/hook.sh
```

Without the hook veil still works for all manual commands — you just won't have automatic history recording.

---

## Commands

### Safety & exploration

| Command | What it does |
|---|---|
| `veil undo` | Reverse the last destructive command — restores files and directories from a snapshot |
| `veil back <N>` | Roll your terminal session back N minutes using filesystem snapshots |
| `veil preview <cmd>` | Show every file a command would touch before you run it |
| `veil whatif <cmd>` | Assess risk before running: HIGH / MEDIUM / SAFE with a reason |
| `veil sandbox <cmd>` | Run a command in an isolated copy of your project; nothing touches your real files |

### History & intelligence

| Command | What it does |
|---|---|
| `veil find <query>` | Fuzzy-search your entire command history |
| `veil why` | Plain-English explanation of what the last command did |
| `veil related <query>` | Find commands similar to a query from your history |
| `veil next` | Suggest the next command based on what usually follows in this context |
| `veil session replay [N]` | Replay the last N commands from this session |

### Time navigation

| Command | What it does |
|---|---|
| `veil rewind [minutes]` | Show terminal state from N minutes ago |
| `veil timeline [limit]` | Browse your snapshot history as a timeline |
| `veil play <time>` | Jump to a specific point in time (e.g., `veil play 30m`) |

### Project intelligence

| Command | What it does |
|---|---|
| `veil status` | Live project briefing — stack, git branch, health |
| `veil audit` | Check for missing dependencies across Rust, Node, Python, Go, Docker |
| `veil analyze` | Full project health report — file metrics, complexity, dead code |
| `veil deps [--format json]` | Dependency graph as ASCII tree or JSON |
| `veil impact <file>` | Show every file that depends on a given file |

### Workflows

| Command | What it does |
|---|---|
| `veil workflow list` | List saved command sequence workflows |
| `veil workflow save <name>` | Detect patterns in recent history and save as a named workflow |

### Bookmarks

| Command | What it does |
|---|---|
| `veil bookmark add <name>` | Save current directory as a named bookmark |
| `veil bookmark list` | List all bookmarks |
| `veil bookmark remove <name>` | Remove a bookmark |
| `veil go <name>` | Jump to a bookmarked directory |

### Remote execution

> Requires OpenSSH client (built into Windows 10+, Linux, macOS).

| Command | What it does |
|---|---|
| `veil remote add <name> <host> <user>` | Register a remote host |
| `veil remote list` | List all registered hosts |
| `veil remote ssh <host> <cmd>` | Execute a command on a remote host |
| `veil remote share <session-id>` | Generate a shareable token for a session replay |
| `veil remote remove <name>` | Remove a registered host |

### Team collaboration

| Command | What it does |
|---|---|
| `veil team setup <name> <type> <url>` | Configure a team remote (`github`, `s3`, `server`) |
| `veil team list` | List team configurations |
| `veil team share bookmark <name>` | Share a bookmark with the team |
| `veil team share workflow <name>` | Share a workflow with the team |
| `veil team pull` | Pull shared bookmarks and workflows |

---

## How it works

Veil is built from 15 isolated engines sharing a single `~/.veil/` directory:

```
your terminal
     │
     ▼
┌──────────────────────────────────────┐
│  veil hook (records every command)   │
└──────────────────┬───────────────────┘
                   │
     ┌─────────────▼─────────────┐
     │     Engine Layer          │
     │                           │
     │  memoir   │ drift         │
     │  phantom  │ trace         │
     │  context  │ bookmarks     │
     │  auditor  │ recorder      │
     │  patterns │ temporal      │
     │  sandbox  │ context_suggest│
     │  analyzer │ remote        │
     │  team                     │
     └─────────────┬─────────────┘
                   │
     ┌─────────────▼─────────────┐
     │  Storage (~/.veil/)       │
     │  memoir.db  sessions.db   │
     │  snapshots/ remotes.db    │
     │  contextual_patterns.db   │
     │  analysis_cache.db        │
     └───────────────────────────┘
```

Each engine is independent with its own SQLite database. Engines fail gracefully — if one breaks, the rest keep working. No external database. No cloud dependency. No telemetry.

---

## Storage

All data lives in `~/.veil/`. You can inspect, back up, or delete any database file independently:

| File | Contents |
|---|---|
| `memoir.db` | Full command history with timestamps, exit codes, directories |
| `sessions.db` | Per-session command sequences for replay |
| `snapshots/` | Filesystem snapshots for undo and rewind |
| `contextual_patterns.db` | Detected workflow patterns |
| `remotes.db` | Remote host registry and shared session tokens |
| `team_sync.db` | Team configuration and shared assets |

---

## Built with

- [Rust](https://www.rust-lang.org/) — zero-overhead core
- [clap](https://github.com/clap-rs/clap) — CLI framework
- [rusqlite](https://github.com/rusqlite/rusqlite) — embedded SQLite (no external DB required)
- [chrono](https://github.com/chronotope/chrono) — timestamps and time arithmetic
- [colored](https://github.com/colored-rs/colored) — terminal output
- [strsim](https://github.com/dguo/strsim-rs) — fuzzy command matching
- [serde](https://serde.rs/) — JSON serialization

No AI/ML dependencies. No cloud required. No telemetry. Fully offline.

---

## License

MIT — free to use, modify, and distribute.

---

<p align="center">Built by <a href="https://github.com/SiphiweRadebe">SiphiweRadebe</a></p>
