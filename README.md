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

- **Blazing Fast**: Native Rust binary compiled for near-zero execution latency.
- **Dual Quota Display**: Displays both 5-Hour and Weekly quotas side-by-side with countdown timers.
- **Responsive Layout**: Dynamically adjusts layout based on terminal width.
- **Rich Diagnostics**: Displays agent state, active model, token counts, git branch, conversation ID, artifacts, subagent count, and background tasks.

## Build from Source

```cmd
git clone https://github.com/vmsysadm/agy-statusline-win.git
cd agy-statusline-win
cargo build --release
powershell .\install.ps1
```

## Uninstallation

Run this command in PowerShell:

```powershell
irm https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/uninstall.ps1 | iex
```

## Credits & License

- Derived from [agy-statusline](https://codeberg.org/jochenkirstaetter/agy-statusline) by **Jochen Kirstätter** ([@jochenkirstaetter](https://codeberg.org/jochenkirstaetter)).
- Licensed under the **MIT License**.
