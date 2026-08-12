# Antigravity CLI Custom Statusline (Windows)

A responsive, high-performance Rust custom statusline for **Antigravity CLI** on Windows.

## Quick Installation

Run this command in PowerShell:

```powershell
irm https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/install.ps1 | iex
```

## Screenshots

| Wide Layout | Medium Layout |
| --- | --- |
| ![Wide Layout](docs/statusline_dual.png) | ![Medium Layout](docs/statusline_single.png) |

## Key Features

- **Blazing Fast**: Native Rust binary compiled for near-zero execution latency (<10ms) with single-pass JSON parsing.
- **Lightweight Process Inspection**: Direct Windows API parent-chain process inspection via `windows-sys` for instant YOLO mode detection without system process scans.
- **Modular Architecture**: Clean 8-module Rust crate structure (`config`, `data`, `theme`, `layout`, `segments`, `yolo`, `ansi`, `main`).
- **Dual Quota Display**: Displays both 5-Hour and Weekly quotas side-by-side with countdown timers.
- **Responsive Layout**: Dynamically adjusts layout based on terminal width.
- **Rich Diagnostics**: Displays agent state, active model, token counts, git branch, conversation ID, artifacts, subagent count, and background tasks.

## Performance & Benchmarks

`agy-statusline-win` is benchmarked over 50 consecutive prompt refreshes using `benchmark.exe`:

| Metric | Baseline | Optimized (v0.7.0) | Speedup |
|---|---|---|---|
| **Min Latency** | ~30.0 ms | **10.84 ms** | **~2.8x faster** |
| **P50 Latency (Median)** | ~45.0 ms | **12.46 ms** | **~3.6x faster** |
| **Avg Latency** | ~48.0 ms | **12.67 ms** | **~3.8x faster** |
| **P95 Latency** | ~60.0 ms | **15.27 ms** | **~3.9x faster** |
| **Max Latency** | ~75.0 ms | **15.40 ms** | **~4.9x faster** |

## Build from Source

```cmd
git clone https://github.com/vmsysadm/agy-statusline-win.git
cd agy-statusline-win
cargo build --release
powershell .\install.ps1
```

## Running Without Nerd Fonts

If you are running on a Windows Server or standard command prompt without a Nerd Font installed, icons may appear as missing characters or double-width mangled symbols. You can run without Nerd Fonts using two methods:

### Option 1: Built-in Emoji Fallback Mode
Set the `USE_NERD_FONTS` environment variable to `false` in PowerShell:

```powershell
[Environment]::SetEnvironmentVariable("USE_NERD_FONTS", "false", "User")
```

### Option 2: Clean ASCII Text Mode (Recommended for Windows Server & conhost)
To avoid any font glyph issues, missing character boxes, or duplicated label text, set `"use_ascii": true` in your `statusline_config.json` or copy the provided `ascii_statusline_config.json` configuration to your `%USERPROFILE%\.gemini\antigravity-cli` directory:

```powershell
Copy-Item "ascii_statusline_config.json" "$env:USERPROFILE\.gemini\antigravity-cli\statusline_config.json" -Force
```

You can also set the `USE_ASCII` environment variable to `true`:

```powershell
[Environment]::SetEnvironmentVariable("USE_ASCII", "true", "User")
```

**Clean ASCII Output Preview:**
```text
! YOLO | * READY | MOD: Gemini 3.6 Flash (medium)                       ART: 0 | SUB: 0 | TASK: 0 | OFF
DIR: ~\code | ID: c23fc741                  CTX:  [==========]4.6% (58.4K/1.0M) | TOK: (48.2K in/10.2K out)
```


## Uninstallation

Run this command in PowerShell:

```powershell
irm https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/uninstall.ps1 | iex
```

## Credits & License

- Derived from [agy-statusline](https://codeberg.org/jochenkirstaetter/agy-statusline) by **Jochen Kirstätter** ([@jochenkirstaetter](https://codeberg.org/jochenkirstaetter)).
- Licensed under the **MIT License**.
