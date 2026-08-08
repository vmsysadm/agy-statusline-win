# Antigravity CLI Custom Statusline (Windows 11)

A responsive, high-performance custom statusline for the **Antigravity CLI** on Windows 11 using PowerShell Core.

## Features

- **Dual Quota Display**: Displays both 5-Hour and Weekly remaining quotas side-by-side (e.g. `96% 5h 4h48m` and `75% wk 100h`).
- **Responsive Layout System**: Dynamically adjusts to terminal window width with parallel right-alignment.
- **Rich Diagnostics**: Agent state, YOLO permissions warning, active cycle mode (`ACCEPT-EDITS` / `PLAN`), active model ID, CWD, Git branch & dirty status, Conversation ID, artifact counts, subagents, and background tasks.
- **Color & Icon Customization**: Fully configurable themes via `statusline_config.json` supporting Truecolor (24-bit RGB) and ANSI escape sequences.

## Quick One-Line Installation

Run the following command in PowerShell on any Windows 11 system:

```powershell
irm https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/install.ps1 | iex
```

## Manual Installation

1. Clone or download this repository.
2. Copy `statusline.ps1` and `statusline_config.json` to your Antigravity CLI directory:
   `%USERPROFILE%\.gemini\antigravity-cli\`
3. Open `%USERPROFILE%\.gemini\antigravity-cli\settings.json` and configure `statusLine`:

```json
{
  "statusLine": {
    "type": "",
    "command": "pwsh.exe -ExecutionPolicy Bypass -File C:\\Users\\YOUR_USERNAME\\.gemini\\antigravity-cli\\statusline.ps1",
    "configPath": "C:\\Users\\YOUR_USERNAME\\.gemini\\antigravity-cli\\statusline_config.json",
    "enabled": true
  }
}
```

## Uninstallation

Run the following command to disable the statusline:

```powershell
irm https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/uninstall.ps1 | iex
```

Or set `"enabled": false` in `%USERPROFILE%\.gemini\antigravity-cli\settings.json`.

## License

MIT License
