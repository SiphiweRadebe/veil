<p align="center">
  <img src="assets/logo.svg" width="400" alt="veil logo"/>
</p>

<p align="center">
  <strong>A thin, intelligent layer over your terminal.</strong><br/>
  Invisible until you need it. Then it saves you.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/built_with-Rust-orange?style=flat-square"/>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=flat-square"/>
  <img src="https://img.shields.io/badge/version-0.1.2-purple?style=flat-square"/>
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square"/>
</p>

---

## What is veil?

Veil wraps your existing terminal and gives it five superpowers it never had:

| Command | What it does |
|---|---|
| `veil undo` | Reverses the last destructive command — files, folders, anything |
| `veil preview <cmd>` | Shows every file a command would touch before you run it |
| `veil why` | Explains what the last command did in plain English |
| `veil status` | Live briefing on your current project — stack, branch, health |
| `veil find <query>` | Searches your personal command knowledge base naturally |
| `veil back <N>` | Rolls your entire terminal session back N minutes |

Veil works silently in the background. You use your terminal exactly as you always have. Veil just watches, learns, and protects.

---

## Install

### Quick Start (Recommended)
**Get pre-built binaries from [GitHub Releases](https://github.com/SiphiweRadebe/veil/releases)** — Windows, Linux, macOS all included.

### Build from Source

#### Windows
**Option A: Automated (Recommended)**
```batch
install-build-tools.bat
```

**Option B: Manual**
1. Download [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/)
2. Install "Desktop development with C++" workload
3. Run: `cargo build --release`

#### Linux / macOS
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/SiphiweRadebe/veil.git
cd veil
cargo build --release
```

#### Docker (Any Platform)
```bash
docker build -t veil . && docker run veil veil --help
```

For detailed build setup, see [BUILD_SETUP.md](BUILD_SETUP.md)

### Installation
```bash
# Linux / macOS
cp target/release/veil ~/.local/bin/veil

# Windows (PowerShell)
copy target\release\veil.exe C:\Users\<you>\.veil\veil.exe
```

### 4. Activate the shell hook

**PowerShell:**
```powershell
# Add to your PowerShell profile
. "C:\path\to\veil\hook.ps1"
```

**bash / zsh** *(coming in v0.2)*:
```bash
source ~/path/to/veil/hook.sh
```

---

## How it works

Veil is built on five engines that share a single event store:
```
your terminal
     │
     ▼
┌─────────────────────────────────────┐
│           command interceptor        │
└──┬──────┬──────┬──────┬─────────────┘
   │      │      │      │
drift  memoir  context  phantom  trace
   │      │      │      │
   └──────┴──────┴──────┘
              │
        event store (SQLite)
              │
        AI layer (v0.5)
```

- **drift** — snapshots your filesystem before every command. Powers `veil undo`.
- **memoir** — records every command into a searchable SQLite database.
- **context** — detects your project type, git branch, and health automatically.
- **phantom** — previews what a command would do before you run it.
- **trace** — explains what a command did to your machine in plain English.

---

## Roadmap

- [x] v0.1 — memoir + drift (history search + undo)
- [x] v0.1 — context (project awareness)
- [x] v0.1 — phantom (danger pattern detection)
- [x] v0.1 — bookmarks, aliases, environment tracking
- [ ] **v1.0 (Major Release)**
  - [ ] **Temporal Navigation** — `veil rewind`, `veil play` (time-travel debugging)
  - [ ] **Safe Exploration** — `veil sandbox`, `veil whatif` (risk-free testing)
  - [ ] **Context Awareness** — `veil related`, `veil workflow`, `veil next` (pattern learning)
  - [ ] **Project Intelligence** — `veil analyze`, `veil deps`, `veil impact` (dependency analysis)
  - [ ] **Shell Sync** — `veil export`, `veil import` (bash/zsh/PowerShell portability)
  - [ ] **Monitoring** — `veil watch`, `veil schedule` (automation & cron)
  - [ ] **Team Collaboration** — `veil sync-team`, `veil broadcast` (team workflows)
  - [ ] **Remote Execution** — `veil ssh`, `veil broadcast` (multi-machine management)
- [ ] v1.1+ — AI layer (Claude API for natural language search)

---

## Built with

- [Rust](https://www.rust-lang.org/) — core engine
- [clap](https://github.com/clap-rs/clap) — CLI framework
- [rusqlite](https://github.com/rusqlite/rusqlite) — SQLite bindings
- [chrono](https://github.com/chronotope/chrono) — time handling
- [colored](https://github.com/colored-rs/colored) — terminal colors

---

## License

MIT — free to use, modify, and distribute.

---

<p align="center">Built by <a href="https://github.com/SiphiweRadebe">SiphiweRadebe</a></p>
