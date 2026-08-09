# Antigravity CLI Custom Statusline (Windows 11) - v0.1

A responsive, high-performance Rust-compiled custom statusline for **Antigravity CLI** on Windows 11.

## Screenshots

### Wide Layout
![Wide Layout](docs/statusline_dual.png)

### Medium Layout
![Medium Layout](docs/statusline_single.png)

## Credits & Acknowledgments

This project is derived from the original [agy-statusline](https://codeberg.org/jochenkirstaetter/agy-statusline) by **Jochen Kirstätter** ([@jochenkirstaetter](https://codeberg.org/jochenkirstaetter)) on Codeberg.

## Features

- **Blazing Fast**: Native Rust binary compiled for low overhead and near-zero execution latency.
- **Dual Quota Display**: Displays both 5-Hour and Weekly remaining quotas side-by-side.
- **Responsive Layout System**: Dynamically adjusts layout based on terminal window width with parallel right-alignment.
- **Rich Diagnostics**: Agent state, cycle mode (`ACCEPT-EDITS` / `PLAN`), active model ID & effort level, CWD, Git branch, Conversation ID, token counts (input/output/total limit), artifact counts, subagent counts, background tasks, and sandbox status.

## Quick One-Line Installation

Run the following command in PowerShell on Windows 11:

```powershell
irm https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/install.ps1 | iex
```

## Build from Source

1. Clone this repository:
   ```cmd
   git clone https://github.com/vmsysadm/agy-statusline-win.git
   cd agy-statusline-win
   ```
2. Build the release binary:
   ```cmd
   cargo build --release
   ```
3. Run `install.ps1` in PowerShell to install the local binary:
   ```powershell
   .\install.ps1
   ```

## Manual Installation

1. Copy `target\release\agy-statusline.exe` to `%USERPROFILE%\.gemini\antigravity-cli\agy-statusline.exe`.
2. Update `%USERPROFILE%\.gemini\antigravity-cli\settings.json`:

```json
{
  "statusLine": {
    "command": "C:\\Users\\YOUR_USERNAME\\.gemini\\antigravity-cli\\agy-statusline.exe",
    "enabled": true
  }
}
```

## Uninstallation

Run the uninstallation script:

```powershell
irm https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/uninstall.ps1 | iex
```

Or set `"enabled": false` in `%USERPROFILE%\.gemini\antigravity-cli\settings.json`.

## License

MIT License
