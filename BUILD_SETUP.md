# Veil Build Setup Instructions

## Quick Start (Windows)

### Option 1: GitHub Actions (Recommended - No Installation)
- Push code to GitHub
- Actions automatically builds on Windows, Linux, macOS
- Releases available in GitHub Releases tab
- Just run: `git push origin main`

### Option 2: Build Tools for Visual Studio (Windows Native)

**Download:**
1. Go to: https://visualstudio.microsoft.com/downloads/
2. Download "Build Tools for Visual Studio 2022" (~500MB installer)
3. Run installer, select:
   - ✅ "Desktop development with C++"
   - ✅ "Windows 10/11 SDK"

**Build:**
```bash
cargo build --release
```

The executable will be at: `target/release/veil.exe`

### Option 3: WSL2 + Linux Build
```bash
# Inside WSL Ubuntu:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
cargo build --release
```

### Option 4: Docker (Cross-Platform)
```bash
docker build -t veil:latest .
docker run veil veil --help
```

## Verification

```bash
# Test the build
cargo test --release

# Run veil
./target/release/veil --help
```

## CI/CD Status
- Windows (MSVC): GitHub Actions
- Linux (GNU): GitHub Actions  
- macOS: GitHub Actions

Current build status: Check `.github/workflows/build.yml`
