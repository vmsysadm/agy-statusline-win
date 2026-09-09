<#
.SYNOPSIS
    End-to-end test: drive a real agy session and assert the tab indicator lights *and clears*.

.DESCRIPTION
    The regression this exists for is "the agent finished but the tab icon persisted", so a test
    that only checks the icon appears is worthless - the failure mode is that it never goes away.
    Every run therefore asserts two things, and the second one is the point:

        1. the indicator lit at some point during the session  (else: inconclusive, the prompt
           never actually spawned the thing under test - not a pass)
        2. it is clear once the turn has ended                 (else: FAIL, the real bug)

    Sampling continues for a grace period *after* agy exits, because that is exactly where the bug
    lives: the work finishes, and the indicator stays lit. Stopping the moment agy returns would
    step over the failure.

    agy is launched with -p (non-interactive). Note that a headless agy deliberately does not paint
    the tab itself - `session::owns_terminal_title` suppresses it so a headless child cannot
    repaint its parent's tab. That does not matter here: this harness does not read the terminal,
    it drives the same probe the real statusline runs, through `--print-title`.

.PARAMETER Scenario
    Which indicator to exercise: `task` (U+2699, background task) or `subagent` (U+1F916).

.PARAMETER Prompt
    Override the built-in prompt. The built-ins are deliberately trivial and side-effect-free -
    a ping loop and a file count - because -p is paired with --dangerously-skip-permissions and
    agy will auto-approve whatever the prompt asks for.

.EXAMPLE
    .\scripts\e2e-tab-test.ps1 -Scenario task
    .\scripts\e2e-tab-test.ps1 -Scenario subagent
#>
[CmdletBinding()]
param(
    [ValidateSet('task', 'subagent', 'both')] [string] $Scenario = 'task',
    [string] $Prompt,
    [int] $TimeoutSeconds = 240,
    [int] $GraceSeconds = 20,
    [double] $IntervalSeconds = 0.5,
    [switch] $KeepLog,
    [string] $Exe = (Join-Path $PSScriptRoot '..\target\debug\agy-statusline.exe')
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if (-not (Test-Path $Exe)) { throw "Statusline binary not found at '$Exe'. Run 'cargo build' first." }
$Exe = (Resolve-Path $Exe).Path
if (-not (Get-Command agy -ErrorAction SilentlyContinue)) { throw 'agy CLI not found on PATH.' }

$brain = Join-Path $env:USERPROFILE '.gemini\antigravity-cli\brain'
if (-not (Test-Path $brain)) { throw "Brain root not found: $brain" }

# Harmless by construction: a ping loop as a timer, and a read-only file count.
if (-not $Prompt) {
    $Prompt = switch ($Scenario) {
        'task'     { 'Run the shell command "ping -n 12 127.0.0.1" as a background task. Wait for it to finish, then reply with exactly: DONE' }
        'subagent' { 'Spawn exactly one subagent whose job is to count the files in the current directory and report the number back to you. Wait for it to finish, then reply with exactly: DONE' }
        'both'     { 'Do these two things so they overlap in time: (1) run the shell command "ping -n 20 127.0.0.1" as a background task, and (2) spawn one subagent to count the files in the current directory. Wait for BOTH to finish, then reply with exactly: DONE' }
    }
}

# Each indicator is checked on its own. Clearing is per-indicator in `get_title_icon`, so the
# combination case is not merely "run both" -- it is the one that catches a completion signal
# being credited to the wrong counter, which two separate single-indicator runs cannot see.
$INDICATORS = @{
    tasks     = @{ Field = 'tasks';     Label = 'U+2699 background task'; Glyph = [char]0x2699 }
    subagents = @{ Field = 'subagents'; Label = 'U+1F916 subagent';       Glyph = [char]::ConvertFromUtf32(0x1F916) }
}
$required = switch ($Scenario) {
    'task'     { @('tasks') }
    'subagent' { @('subagents') }
    'both'     { @('tasks', 'subagents') }
}

Write-Host "Scenario : $Scenario  ($(($required | ForEach-Object { $INDICATORS[$_].Label }) -join ' + '))"
Write-Host "Prompt   : $Prompt"
Write-Host "Watching : $brain`n"

# Conversations that already exist are not ours; the new one that appears is the session we drove.
$before = @(Get-ChildItem $brain -Directory -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name)

$job = Start-Job -ScriptBlock {
    param($p)
    & agy -p $p --dangerously-skip-permissions 2>&1
} -ArgumentList $Prompt

Write-Host ("{0,-9} {1,-9} {2,-8} {3,-6} {4,-6} {5}" -f 'ELAPSED', 'PHASE', 'ICON', 'TASKS', 'SUBAG', 'CODEPOINTS')

$start = Get-Date
$samples = [System.Collections.Generic.List[object]]::new()
$transcript = $null
$lastKey = $null
$exitedAt = $null

function Sample-Icon($path) {
    $payload = @{ transcript_path = $path; agentState = 'working' } | ConvertTo-Json -Compress
    $out = $payload | & $Exe --print-title
    if ($out) { $out | ConvertFrom-Json } else { $null }
}

while ($true) {
    $elapsed = ((Get-Date) - $start).TotalSeconds

    # Bind to the conversation dir that appeared after launch.
    if (-not $transcript) {
        $new = @(Get-ChildItem $brain -Directory -ErrorAction SilentlyContinue |
                 Where-Object { $_.Name -notin $before }) |
               Sort-Object CreationTime -Descending | Select-Object -First 1
        if ($new) {
            $candidate = Join-Path $new.FullName '.system_generated\logs\transcript.jsonl'
            if (Test-Path $candidate) {
                $transcript = $candidate
                Write-Host "  -> session $($new.Name)`n"
            }
        }
    }

    if ($transcript) {
        $r = Sample-Icon $transcript
        if ($r) {
            $phase = if ($exitedAt) { 'after' } else { 'running' }
            $samples.Add([pscustomobject]@{
                Elapsed   = $elapsed
                Phase     = $phase
                Tasks     = [int]$r.tasks
                Subagents = [int]$r.subagents
                Icon      = $r.icon
            })

            $key = "$($r.tasks)|$($r.subagents)|$phase"
            if ($key -ne $lastKey) {
                Write-Host ("{0,-9:N1} {1,-9} {2,-8} {3,-6} {4,-6} {5}" -f
                    $elapsed, $phase, $r.icon, $r.tasks, $r.subagents, ($r.codepoints -join ' '))
                $lastKey = $key
            }
        }
    }

    if (-not $exitedAt -and $job.State -ne 'Running') {
        $exitedAt = Get-Date
        Write-Host "  -> agy exited after $([int]$elapsed)s; sampling ${GraceSeconds}s more (this is where the bug hides)`n"
    }
    if ($exitedAt -and ((Get-Date) - $exitedAt).TotalSeconds -ge $GraceSeconds) { break }
    if ($elapsed -ge $TimeoutSeconds) { Write-Warning "Timed out after ${TimeoutSeconds}s."; break }

    Start-Sleep -Seconds $IntervalSeconds
}

$reply = (Receive-Job $job -ErrorAction SilentlyContinue) -join "`n"
Stop-Job $job -ErrorAction SilentlyContinue
Remove-Job $job -Force -ErrorAction SilentlyContinue

Write-Host "`n--- agy reply ---"
Write-Host ($reply.Trim() | Select-Object -First 1)
Write-Host '-----------------'

if (-not $transcript) { Write-Host "`nFAIL: never located a transcript for the session." -ForegroundColor Red; exit 2 }

$after = @($samples | Where-Object { $_.Phase -eq 'after' })
$final = $samples[-1]

Write-Host "`nsamples: $($samples.Count)  (post-exit $($after.Count))"

$inconclusive = @()
$failed = @()

foreach ($name in $required) {
    $ind  = $INDICATORS[$name]
    $prop = if ($name -eq 'tasks') { 'Tasks' } else { 'Subagents' }

    $lit    = @($samples | Where-Object { $_.$prop -gt 0 })
    $endLit = @($after   | Where-Object { $_.$prop -gt 0 })

    if ($lit.Count -eq 0) {
        Write-Host "  $($ind.Glyph) $($ind.Label): never lit" -ForegroundColor Yellow
        $inconclusive += $ind.Label
        continue
    }

    $cleared = @($samples | Where-Object { $_.Elapsed -gt $lit[-1].Elapsed })
    if ($endLit.Count -gt 0) {
        Write-Host ("  $($ind.Glyph) $($ind.Label): lit {0:N1}s, STILL LIT in {1}/{2} post-exit samples" -f
            $lit[0].Elapsed, $endLit.Count, $after.Count) -ForegroundColor Red
        $failed += $ind.Label
    } else {
        Write-Host ("  $($ind.Glyph) $($ind.Label): lit {0:N1}s -> cleared {1:N1}s, clear in all {2} post-exit samples" -f
            $lit[0].Elapsed, $lit[-1].Elapsed, $after.Count) -ForegroundColor Green
    }
}

# The combination's own assertion. Two indicators that merely took turns would pass the per-
# indicator checks above while never exercising concurrent tracking at all.
if ($Scenario -eq 'both') {
    $overlap = @($samples | Where-Object { $_.Tasks -gt 0 -and $_.Subagents -gt 0 })
    if ($overlap.Count -eq 0) {
        Write-Host "  (!) never lit simultaneously - ran sequentially, so the combination was not exercised" -ForegroundColor Yellow
        $inconclusive += 'concurrent overlap'
    } else {
        Write-Host ("  both lit together for {0} sample(s), from {1:N1}s to {2:N1}s" -f
            $overlap.Count, $overlap[0].Elapsed, $overlap[-1].Elapsed) -ForegroundColor Green
    }
}

if ($failed.Count -gt 0) {
    Write-Host "`nFAIL: still lit after the turn ended -> $($failed -join ', ')" -ForegroundColor Red
    Write-Host "Final icon: $($final.Icon)  (tasks=$($final.Tasks) subagents=$($final.Subagents))"
    Write-Host "This is the stuck-tab regression. Capture it:"
    Write-Host "  .\scripts\capture-fixture.ps1 -Transcript '$transcript' -Name <kebab-name>"
    exit 1
}

if ($inconclusive.Count -gt 0) {
    Write-Host "`nINCONCLUSIVE: $($inconclusive -join ', ') - the prompt did not produce it, so nothing was proven." -ForegroundColor Yellow
    Write-Host "Transcript: $transcript"
    exit 3
}

Write-Host "`nPASS: every indicator lit during the session and cleared once it ended. Final icon: $($final.Icon)" -ForegroundColor Green
if ($KeepLog) { Write-Host "Transcript: $transcript" }
