<#
.SYNOPSIS
    Replay a captured agy transcript prefix-by-prefix and print the tab-icon timeline.

.DESCRIPTION
    Turns a one-off "the gear stayed on the tab" observation into something reproducible. The
    session is copied into a scratch brain tree and replayed step by step: for each prefix of the
    parent transcript, the icon is resolved through `agy-statusline.exe --print-title`, giving the
    tab icon as it would have appeared at that moment.

    The invariant worth asserting on any finished session: by the final step the icon carries
    neither U+2699 (background task) nor U+1F916 (subagent). Pass -AssertClearAtEnd to exit
    non-zero when it does not.

    The replay is time-faithful, which matters because both indicators are decided from state
    that a naive file copy destroys:

      1. Subagent transcripts are reconstructed as of each replay instant, using the `created_at`
         on every step. Truncating only the parent would leave each subagent frozen at its
         end-of-session state, so a subagent that was working at step N reads as finished and the
         robot indicator never lights - hiding exactly the bug being hunted.

      2. mtimes are re-stamped per step. `subagent_is_working` compares a subagent transcript's
         mtime against SystemTime::now() with a 90s window (SUBAGENT_ACTIVE_WINDOW), so an
         hour-old capture otherwise reads as "everything finished" and a stuck session replays
         clean. Each step stamps the age the subagent actually had at that point.

      3. A subagent that had not yet written its first step has *no* transcript at that instant,
         and the probe counts a missing transcript as working by design (the parent records the
         spawn before the child writes). The replay deletes rather than empties the file so this
         path is exercised honestly.

.PARAMETER Transcript
    Path to a captured transcript.jsonl. Its whole conversation directory is copied, along with
    every sibling UUID conversation it spawns.

.PARAMETER Steps
    Roughly how many points to sample along the transcript. The final step is always sampled.

.EXAMPLE
    .\scripts\replay-transcript.ps1 -Transcript "$env:USERPROFILE\.gemini\antigravity-cli\brain\<id>\.system_generated\logs\transcript.jsonl" -AssertClearAtEnd
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Transcript,
    [int] $Steps = 120,
    [switch] $AssertClearAtEnd,
    [switch] $KeepScratch,
    [switch] $AllSteps,
    [string] $Exe = (Join-Path $PSScriptRoot '..\target\debug\agy-statusline.exe')
)

$ErrorActionPreference = 'Stop'

# The exe writes UTF-8; without this its icons decode through the console's legacy code page.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if (-not (Test-Path $Transcript)) { throw "Transcript not found: $Transcript" }
if (-not (Test-Path $Exe)) { throw "Statusline binary not found at '$Exe'. Run 'cargo build' first." }
$Exe = (Resolve-Path $Exe).Path
$Transcript = (Resolve-Path $Transcript).Path

# <brain>/<conv>/.system_generated/logs/transcript.jsonl
$logsDir  = Split-Path $Transcript -Parent
$convDir  = Split-Path (Split-Path $logsDir -Parent) -Parent
$brainDir = Split-Path $convDir -Parent
$convId   = Split-Path $convDir -Leaf

# `created_at` is "MM/dd/yyyy HH:mm:ss". Steps written in the same second are common, so this is
# only ever used for ordering and for ages measured in tens of seconds - both well inside its
# resolution.
function Get-StepTime([string] $line) {
    if ($line -notmatch '"created_at"\s*:\s*"([^"]+)"') { return $null }
    try { [datetime]::Parse($Matches[1], [cultureinfo]::InvariantCulture) } catch { $null }
}

$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("agy-replay-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
$destBrain = Join-Path $scratch 'brain'
New-Item -ItemType Directory -Path $destBrain -Force | Out-Null

Write-Host "Source conversation: $convId"
Copy-Item $convDir -Destination (Join-Path $destBrain $convId) -Recurse -Force

# Any UUID the parent names is a candidate subagent; keep the ones that exist as sibling
# conversations. Their step lines are held in memory and re-materialised per replay step.
$uuidPattern = '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}'
$subIds = Select-String -Path $Transcript -Pattern $uuidPattern -AllMatches |
    ForEach-Object { $_.Matches.Value } |
    Sort-Object -Unique |
    Where-Object { $_ -ne $convId -and (Test-Path (Join-Path $brainDir "$_\.system_generated\logs\transcript.jsonl")) }

$subs = @{}
foreach ($id in $subIds) {
    $srcLines = Get-Content (Join-Path $brainDir "$id\.system_generated\logs\transcript.jsonl") -ReadCount 0
    $dstLogs = Join-Path $destBrain "$id\.system_generated\logs"
    New-Item -ItemType Directory -Path $dstLogs -Force | Out-Null
    $subs[$id] = [pscustomobject]@{
        Lines = $srcLines
        Times = @($srcLines | ForEach-Object { Get-StepTime $_ })
        Path  = Join-Path $dstLogs 'transcript.jsonl'
    }
}
Write-Host "Subagent conversations: $($subs.Count)"

$replayTranscript = Join-Path $destBrain "$convId\.system_generated\logs\transcript.jsonl"
$lines = Get-Content $Transcript -ReadCount 0
$times = @($lines | ForEach-Object { Get-StepTime $_ })
$total = $lines.Count
Write-Host "Transcript lines      : $total`n"

# Sample evenly rather than replaying every prefix: each step rewrites the files, and a long
# session runs to tens of thousands of lines.
$stepSize = [Math]::Max(1, [int][Math]::Ceiling($total / [Math]::Max(1, $Steps)))
$marks = @(1..$total | Where-Object { $_ % $stepSize -eq 0 })
if ($marks[-1] -ne $total) { $marks += $total }

$payload = @{ transcript_path = $replayTranscript; agentState = 'working' } | ConvertTo-Json -Compress

# Rebuild every subagent transcript as it stood at $asOf, with the age it had then.
function Set-SubagentState([datetime] $asOf) {
    $now = Get-Date
    foreach ($id in $subs.Keys) {
        $s = $subs[$id]
        $keep = 0
        $lastTime = $null
        for ($i = 0; $i -lt $s.Lines.Count; $i++) {
            if ($null -ne $s.Times[$i] -and $s.Times[$i] -gt $asOf) { break }
            $keep = $i + 1
            if ($null -ne $s.Times[$i]) { $lastTime = $s.Times[$i] }
        }

        if ($keep -eq 0) {
            # Spawned but not yet writing: the probe must see no file at all, not an empty one.
            Remove-Item $s.Path -Force -ErrorAction SilentlyContinue
            continue
        }

        Set-Content -Path $s.Path -Value $s.Lines[0..($keep - 1)] -Encoding utf8
        if ($null -ne $lastTime) {
            $age = $asOf - $lastTime
            if ($age.TotalSeconds -lt 0) { $age = [TimeSpan]::Zero }
            (Get-Item $s.Path).LastWriteTime = $now - $age
        }
    }
}

Write-Host ("{0,-8} {1,-10} {2,-8} {3,-6} {4,-6} {5}" -f 'LINE', 'TIME', 'ICON', 'TASKS', 'SUBAG', 'CODEPOINTS')

$last = $null
$final = $null
foreach ($n in $marks) {
    $asOf = $times[$n - 1]
    if ($null -eq $asOf) { $asOf = ($times[0..($n - 1)] | Where-Object { $_ } | Select-Object -Last 1) }

    Set-Content -Path $replayTranscript -Value $lines[0..($n - 1)] -Encoding utf8
    if ($subs.Count -gt 0 -and $null -ne $asOf) { Set-SubagentState $asOf }

    $out = $payload | & $Exe --print-title
    if (-not $out) { continue }
    $r = $out | ConvertFrom-Json
    $final = $r

    $key = "$($r.tasks)|$($r.subagents)|$($r.blocked)"
    if ($AllSteps -or $key -ne $last) {
        $stamp = if ($asOf) { $asOf.ToString('HH:mm:ss') } else { '--:--:--' }
        Write-Host ("{0,-8} {1,-10} {2,-8} {3,-6} {4,-6} {5}" -f
            $n, $stamp, $r.icon, $r.tasks, $r.subagents, ($r.codepoints -join ' '))
        $last = $key
    }
}

Write-Host ''
$stuck = @()
if ($final.tasks -gt 0)     { $stuck += "$($final.tasks) background task(s) U+2699" }
if ($final.subagents -gt 0) { $stuck += "$($final.subagents) subagent(s) U+1F916" }

if ($stuck.Count -gt 0) {
    Write-Host "STUCK at end of transcript: $($stuck -join ', ')" -ForegroundColor Red
    Write-Host "Final icon: $($final.icon)  ($($final.codepoints -join ' '))"
} else {
    Write-Host "Clean at end of transcript. Final icon: $($final.icon)" -ForegroundColor Green
}

if ($KeepScratch) { Write-Host "`nScratch kept at: $scratch" }
else { Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue }

if ($AssertClearAtEnd -and $stuck.Count -gt 0) { exit 1 }
