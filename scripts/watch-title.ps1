<#
.SYNOPSIS
    Record what the terminal tab icon resolves to, once a second, while a real agy session runs.

.DESCRIPTION
    The statusline emits its OSC 0 title to CONOUT$, so nothing downstream can capture it and the
    only way to check the tab has ever been to look at it. This drives the same code path through
    `agy-statusline.exe --print-title`, which reports the icon on stdout instead, and logs a
    timeline you can read after the fact.

    Console output shows only *changes*, so a stuck indicator appears as a line that never gets a
    successor. The JSONL log keeps every sample plus the transcript size at that instant, which is
    what tells you where to cut a fixture (see replay-transcript.ps1).

.PARAMETER ConversationId
    Pin a specific conversation. Defaults to whichever transcript under the brain root was
    modified most recently, re-resolved on every sample so it follows the session you are in.

.PARAMETER AgentState
    The `agentState` to report in the synthetic payload. The indicators under test (U+2699 tasks,
    U+1F916 subagents) are appended for every state except the waiting ones, so the default of
    "working" exercises them. Use "idle" to confirm they still clear against a resting base icon.

.PARAMETER LogPath
    Where to append samples. Defaults to a timestamped file under scripts/logs/.

.EXAMPLE
    .\scripts\watch-title.ps1
    # ...then, in the agy session: spawn a background task, let it finish. Watch for the U+2699
    # line to be followed by one without it.
#>
[CmdletBinding()]
param(
    [string] $ConversationId,
    [string] $AgentState = 'working',
    [string] $LogPath,
    [double] $IntervalSeconds = 1.0,
    [string] $Exe = (Join-Path $PSScriptRoot '..\target\debug\agy-statusline.exe')
)

$ErrorActionPreference = 'Stop'

# The exe writes UTF-8; without this PowerShell decodes its stdout in the console's legacy code
# page and every icon lands in the log as mojibake (🔆 -> "=���"). The `codepoints` field survives
# either way, being ASCII, but the icon itself is what a human scans the log for.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if (-not (Test-Path $Exe)) {
    throw "Statusline binary not found at '$Exe'. Run 'cargo build' first, or pass -Exe."
}
$Exe = (Resolve-Path $Exe).Path

$brain = Join-Path $env:USERPROFILE '.gemini\antigravity-cli\brain'
if (-not (Test-Path $brain)) { throw "Brain root not found: $brain" }

if (-not $LogPath) {
    $logDir = Join-Path $PSScriptRoot 'logs'
    if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }
    $LogPath = Join-Path $logDir ("title-{0:yyyyMMdd-HHmmss}.jsonl" -f (Get-Date))
}

# Resolve the transcript fresh each sample: a session started after the watcher must be picked up,
# and pinning one path at startup is how you end up watching a conversation you already left.
function Resolve-Transcript {
    if ($ConversationId) {
        $p = Join-Path $brain "$ConversationId\.system_generated\logs\transcript.jsonl"
        return (Test-Path $p) ? $p : $null
    }
    Get-ChildItem $brain -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName '.system_generated\logs\transcript.jsonl' } |
        Where-Object { Test-Path $_ } |
        Get-Item |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}

Write-Host "Watching  : $(if ($ConversationId) { $ConversationId } else { '<newest transcript>' })"
Write-Host "Logging to: $LogPath"
Write-Host "Ctrl+C to stop. Only icon changes are printed.`n"
Write-Host ("{0,-12} {1,-8} {2,-6} {3,-6} {4}" -f 'TIME', 'ICON', 'TASKS', 'SUBAG', 'CODEPOINTS')

$last = $null
while ($true) {
    $transcript = Resolve-Transcript
    if ($transcript) {
        $item = Get-Item $transcript
        # ConvertTo-Json escapes the backslashes; -Compress keeps it to the single line the exe reads.
        $payload = @{ transcript_path = $transcript; agentState = $AgentState } | ConvertTo-Json -Compress

        $out = $payload | & $Exe --print-title
        if ($LASTEXITCODE -eq 0 -and $out) {
            $r = $out | ConvertFrom-Json
            $stamp = Get-Date -Format 'HH:mm:ss.fff'

            $sample = [ordered]@{
                time          = (Get-Date).ToString('o')
                icon          = $r.icon
                codepoints    = $r.codepoints
                tasks         = $r.tasks
                subagents     = $r.subagents
                blocked       = $r.blocked
                state         = $r.state
                title_enabled = $r.title_enabled
                # <brain>/<conv>/.system_generated/logs/transcript.jsonl -> <conv>
                conversation  = Split-Path (Split-Path (Split-Path (Split-Path $transcript -Parent) -Parent) -Parent) -Leaf
                bytes         = $item.Length
                lines         = (Get-Content $transcript -ReadCount 0).Count
            }
            ($sample | ConvertTo-Json -Compress -Depth 4) | Add-Content -Path $LogPath -Encoding utf8

            # Key against the indicators, not the whole icon: the working base icon pulses
            # between 🔆 and 🔅 in normal operation and would otherwise print every sample.
            $key = "$($r.tasks)|$($r.subagents)|$($r.blocked)|$($r.state)"
            if ($key -ne $last) {
                Write-Host ("{0,-12} {1,-8} {2,-6} {3,-6} {4}" -f
                    $stamp, $r.icon, $r.tasks, $r.subagents, ($r.codepoints -join ' '))
                $last = $key
            }
            if (-not $r.title_enabled) {
                Write-Warning 'terminalTitle.enabled is false in statusline_config.json - the tab will not move regardless of this icon.'
            }
        }
    }
    Start-Sleep -Seconds $IntervalSeconds
}
