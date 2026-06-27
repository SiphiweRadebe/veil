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
  <img src="https://img.shields.io/badge/version-1.0.0-purple?style=flat-square"/>
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square"/>
</p>

---

## Why veil?

Your terminal doesn't protect you. It executes what you type, no questions asked. Veil adds a persistent intelligence layer that watches, learns, and intervenes when you need it.

- **No more accidents** — undo destructive commands, sandbox experiments, predict side effects before running
- **No more forgotten commands** — full searchable history, smart suggestions, pattern detection
- **No more context switching** — project awareness, dependency graphs, environment tracking in one tool
- **No more repetition** — detect your workflows and replay them, share them with your team

Veil works alongside your existing shell. You don't change how you work; veil just makes every command smarter.

---

## Get veil

### Option 1: Download a binary (fastest)

Grab a pre-built binary from **[GitHub Releases](https://github.com/SiphiweRadebe/veil/releases)** — Windows `.exe`, Linux, and macOS binaries are all built automatically on every release.

```powershell
# Windows — move to somewhere in your PATH
move veil.exe C:\Users\<you>\bin\veil.exe

# Linux / macOS
chmod +x veil && mv veil ~/.local/bin/veil
```

### Option 2: Build from source

**Requirements:** Rust 1.70+ (`rustup.rs`) and a C linker (MSVC on Windows, `gcc` on Linux/macOS).

```bash
git clone https://github.com/SiphiweRadebe/veil.git
cd veil
cargo build --release
# Binary is at: target/release/veil (or veil.exe on Windows)
```

**Windows build note:** You need the MSVC linker. Either:
- Install [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/) with the "Desktop development with C++" workload, or
- Run `install-build-tools.bat` in this repo for an automated setup

**Docker (any platform):**
```bash
docker build -t veil .
docker run veil veil --help
```

See [BUILD_SETUP.md](BUILD_SETUP.md) for detailed platform-specific instructions.

### Option 3: Shell hook (recommended after install)

Add veil's hook to your shell so it can record command history automatically:

**PowerShell** — add to your `$PROFILE`:
```powershell
. "C:\path\to\veil\hook.ps1"
```

**bash / zsh** — add to `.bashrc` / `.zshrc`:
```bash
source ~/path/to/veil/hook.sh
```

Without the hook, veil still works for all manual commands — you just won't have automatic history recording.

---

## Commands

### Safety & exploration

| Command | What it does |
|---|---|
| `veil undo` | Reverse the last destructive command — restores files, directories, anything snapshoted |
| `veil preview <cmd>` | Show every file a command would touch before you run it |
| `veil whatif <cmd>` | Assess risk level before running: HIGH / MEDIUM / SAFE with a reason |
| `veil sandbox <cmd>` | Run a command in an isolated copy of your project; nothing touches your real files |
| `veil back <N>` | Roll your terminal session back N minutes using filesystem snapshots |

### History & search

| Command | What it does |
|---|---|
| `veil find <query>` | Fuzzy-search your entire command history naturally |
| `veil why` | Plain-English explanation of what the last command did |
| `veil related <query>` | Find commands similar to a query from your history |
| `veil next` | Suggest the next command based on what you usually run after the current one |
| `veil session replay [N]` | Replay your last N commands from this session |

### Time navigation

| Command | What it does |
|---|---|
| `veil rewind [minutes]` | Show terminal state from N minutes ago |
| `veil timeline [limit]` | Browse your snapshot history as a timeline |
| `veil play <time>` | Jump to a specific point in time (e.g., `veil play 30m`) |

### Project intelligence

| Command | What it does |
|---|---|
| `veil status` | Live project briefing — stack, git branch, health, open issues |
| `veil audit` | Check for missing dependencies across Rust, Node, Python, Go, Docker |
| `veil analyze` | Full project health report — file metrics, complexity, dead code detection |
| `veil deps [--format json]` | Dependency graph as ASCII tree or JSON |
| `veil impact <file>` | Show every file that depends on a given file |

### Workflows & automation

| Command | What it does |
|---|---|
| `veil workflow list` | List saved command sequence workflows |
| `veil workflow save <name>` | Detect patterns in your recent history and save as a named workflow |
| `veil watch add <name> <pattern> <cmd>` | Run a command whenever files matching a pattern change |
| `veil watch list` | List all registered file watchers |
| `veil watch run <name>` | Start a file watcher |
| `veil schedule add <name> <cron> <cmd>` | Schedule a command on a cron expression (`0 9 * * *`, `*/5 * * * *`) |
| `veil schedule list` | List all scheduled tasks |
| `veil schedule run <name>` | Run a scheduled task in the foreground |

### Aliases & shell sync

| Command | What it does |
|---|---|
| `veil alias add <name> <cmd>` | Create a named alias stored in veil's database |
| `veil alias list` | List all aliases |
| `veil alias suggest` | Suggest aliases for commands you run frequently |
| `veil export <bash\|zsh\|powershell>` | Export your aliases to shell config format |
| `veil import <bash\|zsh\|powershell>` | Import existing aliases from your shell config |
| `veil sync-shell <shell>` | Bi-directional sync between veil and your shell |

### Environment tracking

| Command | What it does |
|---|---|
| `veil env capture` | Snapshot your current environment variables as a baseline |
| `veil env diff` | Show what changed since the last baseline |

### Remote execution

> Requires OpenSSH client installed (built into Windows 10+, Linux, macOS).

| Command | What it does |
|---|---|
| `veil remote add <name> <host> <user>` | Register a remote host |
| `veil remote list` | List all registered hosts |
| `veil remote ssh <host> <cmd>` | Execute a command on a remote host |
| `veil remote broadcast <pattern> <cmd>` | Run a command on all hosts matching a name/tag pattern |
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

Veil is built from 20 isolated engines sharing a single `~/.veil/` data directory:

```
your terminal
     │
     ▼
┌─────────────────────────────────────────────────────┐
│  veil hook (records every command automatically)    │
└──────────────────────────┬──────────────────────────┘
                           │
     ┌─────────────────────▼──────────────────────┐
     │           Engine Layer (/src/engines/)      │
     │                                            │
     │  memoir  │ drift    │ phantom │ trace       │
     │  context │ bookmarks│ auditor │ recorder    │
     │  envoy   │ sage     │ patterns│ temporal    │
     │  sandbox │ context_ │ analyzer│ sync        │
     │          │  suggest │         │             │
     │  monitor │ schedule │ team    │ remote      │
     └──────────────────────┬─────────────────────┘
                            │
     ┌──────────────────────▼─────────────────────┐
     │  Storage Layer (~/.veil/)                  │
     │  memoir.db │ sessions.db │ aliases.db      │
     │  snapshots/ │ contextual_patterns.db       │
     │  monitoring.db │ scheduling.db │ remotes.db│
     └────────────────────────────────────────────┘
```

Each engine is independent with its own SQLite database. Engines fail gracefully — if one breaks, the rest keep working.

---

## Storage

All data lives in `~/.veil/`. You can inspect, back up, or delete any database file:

| File | Contents |
|---|---|
| `memoir.db` | Full command history with timestamps, exit codes, directories |
| `sessions.db` | Per-session command sequences for replay |
| `snapshots/` | Filesystem snapshots for undo and rewind |
| `aliases.db` | Your aliases and import history |
| `contextual_patterns.db` | Detected workflow patterns |
| `monitoring.db` | File watcher registrations and event log |
| `scheduling.db` | Scheduled task definitions and run history |
| `team_sync.db` | Team configuration and shared assets |
| `remotes.db` | Remote host registry and shared session tokens |

---

## Built with

- [Rust](https://www.rust-lang.org/) — zero-overhead core
- [clap](https://github.com/clap-rs/clap) — CLI framework
- [rusqlite](https://github.com/rusqlite/rusqlite) — embedded SQLite (no external DB required)
- [chrono](https://github.com/chronotope/chrono) — timestamps and time arithmetic
- [colored](https://github.com/colored-rs/colored) — terminal output
- [strsim](https://github.com/dguo/strsim-rs) — fuzzy command matching
- [serde](https://serde.rs/) — JSON serialization

No AI/ML dependencies. No cloud required. No telemetry. Everything runs locally.

---

## License

MIT — free to use, modify, and distribute.

---

<p align="center">Built by <a href="https://github.com/SiphiweRadebe">SiphiweRadebe</a></p>
