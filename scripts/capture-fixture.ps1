<#
.SYNOPSIS
    Freeze a real agy session into a committed test fixture.

.DESCRIPTION
    The step that turns "I saw the gear stick on the tab" into a regression test that runs forever.
    Copies a session's transcript, its per-task logs and every subagent conversation it spawned
    into tests/fixtures/<Name>/, alongside a meta.json recording what the icon is expected to be.

    Ages, not mtimes. A committed fixture's file timestamps are whatever git checkout happens to
    write, but `subagent_is_working` measures a subagent transcript's age against a 90s window - so
    the fixture would mean something different on every machine. meta.json therefore stores each
    subagent's age in seconds as of the end of the session, computed from the `created_at` stamps
    in the transcripts, and the Rust test re-applies it with `set_modified` at run time.

.PARAMETER Expect
    What the tab should show at the end of this session. Defaults to "clean" - no U+2699, no
    U+1F916 - which is the invariant for any session whose work has finished. Use -Expect stuck
    to capture a session that is *legitimately* still busy at the point of capture.

.PARAMETER Trim
    Keep only the last N lines of the parent transcript. Fixtures should be small; the probe only
    reads the last 256 KB anyway. Subagent transcripts are never trimmed - they are small, and
    their tails are what decides the robot indicator.

.EXAMPLE
    .\scripts\capture-fixture.ps1 -Transcript <path> -Name gear-stuck-after-timer-cancel
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Transcript,
    [Parameter(Mandatory)] [string] $Name,
    [ValidateSet('clean', 'stuck')] [string] $Expect = 'clean',
    [int] $Trim = 0,
    [string] $Note = ''
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $Transcript)) { throw "Transcript not found: $Transcript" }
if ($Name -notmatch '^[a-z0-9][a-z0-9-]*$') { throw "Name must be kebab-case: '$Name'" }

$Transcript = (Resolve-Path $Transcript).Path
$sysDir   = Split-Path (Split-Path $Transcript -Parent) -Parent
$convDir  = Split-Path $sysDir -Parent
$brainDir = Split-Path $convDir -Parent
$convId   = Split-Path $convDir -Leaf

$root = Join-Path $PSScriptRoot "..\tests\fixtures\$Name"
if (Test-Path $root) { throw "Fixture already exists: $root" }

function Get-StepTime([string] $line) {
    if ($line -notmatch '"created_at"\s*:\s*"([^"]+)"') { return $null }
    try { [datetime]::Parse($Matches[1], [cultureinfo]::InvariantCulture) } catch { $null }
}

<#
    Rebuild a transcript step keeping ONLY what the probe reads.

    A real transcript is a verbatim record of a session: prompts, tool output, file contents, and
    -- because a statusline payload is itself something an agent may print -- account fields like
    the user's email, plan tier and quota. None of that is input to `probe_session_activity`, and
    a fixture is a file committed to a public repository. So the content is not scrubbed by
    pattern (which can only remove what someone thought to look for); it is discarded, and the
    handful of marker substrings the probe matches on are put back.

    What must survive, and why:
      - the task start phrase and its id      -> counts a task as started
      - `sender=<conv>/task-N`                -> retires it
      - the subagent spawn phrase + UUID      -> counts a subagent
      - the `Successfully killed ... subagent` phrase -> retires all of them
      - `ask_question`                        -> drives the blocked-on-question icon
      - type / status / tool_calls presence   -> decides `transcript_turn_ended`
#>
function Redact-Line([string] $line) {
    $keep = [System.Collections.Generic.List[string]]::new()

    foreach ($m in [regex]::Matches($line, 'Tool is running as a background task with task id: [^\s"\\]+')) {
        $keep.Add($m.Value)
    }
    foreach ($m in [regex]::Matches($line, 'sender=[^\s"\\]+')) { $keep.Add("[Message] $($m.Value) content=redacted") }
    if ($line -match 'Successfully killed \d+ subagent') { $keep.Add($Matches[0] + '(s) and their descendants.') }

    if ($line -match 'Created the following subagents' -or $line -match '"type":"INVOKE_SUBAGENT"') {
        foreach ($m in [regex]::Matches($line, '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}')) {
            $keep.Add("Created the following subagents:`n{`n  ""conversationId"": ""$($m.Value)""`n}")
        }
    }
    if ($line -match '"ask_question"') { $keep.Add('"ask_question"') }

    $obj = [ordered]@{}
    foreach ($f in 'step_index', 'source', 'type', 'status', 'created_at') {
        if ($line -match "`"$f`"\s*:\s*(""[^""]*""|\d+)") {
            $obj[$f] = $Matches[1].Trim('"')
        }
    }
    if ($obj.Contains('step_index')) { $obj['step_index'] = [int]$obj['step_index'] }
    $obj['content'] = ($keep -join "`n")

    # `transcript_turn_ended` requires the *absence* of tool_calls, so a step that had them must
    # still say so or every subagent would read as finished the moment its turn was mid-flight.
    if ($line -match '"tool_calls"') { $obj['tool_calls'] = @('redacted') }

    $obj | ConvertTo-Json -Compress -Depth 5
}

function Get-LastStepTime([string] $path) {
    $t = $null
    foreach ($l in (Get-Content $path -ReadCount 0)) { $x = Get-StepTime $l; if ($x) { $t = $x } }
    $t
}

# --- parent transcript -------------------------------------------------------
$lines = Get-Content $Transcript -ReadCount 0
if ($Trim -gt 0 -and $lines.Count -gt $Trim) { $lines = $lines[-$Trim..-1] }

$destLogs = Join-Path $root "$convId\.system_generated\logs"
New-Item -ItemType Directory -Path $destLogs -Force | Out-Null
Set-Content -Path (Join-Path $destLogs 'transcript.jsonl') -Value ($lines | ForEach-Object { Redact-Line $_ }) -Encoding utf8

$sessionEnd = $null
foreach ($l in $lines) { $x = Get-StepTime $l; if ($x) { $sessionEnd = $x } }
if (-not $sessionEnd) { $sessionEnd = (Get-Item $Transcript).LastWriteTime }

# --- per-task logs: load-bearing for clearing U+2699 -------------------------
$srcTasks = Join-Path $sysDir 'tasks'
$taskCount = 0
if (Test-Path $srcTasks) {
    $destTasks = Join-Path $root "$convId\.system_generated\tasks"
    New-Item -ItemType Directory -Path $destTasks -Force | Out-Null
    # A task log is free-form prose (the timer's prompt, its early-termination condition, whatever
    # the tool printed). `finished_per_task_log` only ever looks for TASK_LOG_DONE_MARKERS, so keep
    # the lines carrying one and drop the rest.
    $markers = 'Timer cancelled', 'Timer fired'
    foreach ($f in Get-ChildItem $srcTasks -Filter '*.log' -File) {
        $kept = @(Get-Content $f.FullName -ReadCount 0 | Where-Object {
            $l = $_; ($markers | Where-Object { $l -like "*$_*" }).Count -gt 0
        }) | ForEach-Object {
            # Even a kept line can trail a reason naming ids or prompts; cut at the marker.
            foreach ($m in $markers) { if ($_ -like "*$m*") { return $m } }
        }
        Set-Content -Path (Join-Path $destTasks $f.Name) -Value $kept -Encoding utf8
        $taskCount++
    }
}

# --- subagent conversations --------------------------------------------------
$uuidPattern = '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}'
$ages = [ordered]@{}

# Only UUIDs on a spawn line are subagents, mirroring `probe_session_activity`. A transcript
# mentions plenty of other conversation ids -- quoted logs, paths, prose -- and the probe ignores
# them by design. Copying those too would bulk the fixture up with unrelated sessions and, worse,
# imply the fixture is testing something it is not.
$spawnLines = $lines | Where-Object {
    ($_ -match 'Created the following subagents' -or $_ -match '"type":"INVOKE_SUBAGENT"') -and
    $_ -notmatch 'File Path:' -and $_ -notmatch 'Output:'
}
$subIds = ($spawnLines | Select-String -Pattern $uuidPattern -AllMatches).Matches.Value |
    Sort-Object -Unique |
    Where-Object { $_ -ne $convId -and (Test-Path (Join-Path $brainDir "$_\.system_generated\logs\transcript.jsonl")) }

foreach ($id in $subIds) {
    $src = Join-Path $brainDir "$id\.system_generated\logs\transcript.jsonl"
    $dstLogs = Join-Path $root "$id\.system_generated\logs"
    New-Item -ItemType Directory -Path $dstLogs -Force | Out-Null
    Set-Content -Path (Join-Path $dstLogs 'transcript.jsonl') `
        -Value ((Get-Content $src -ReadCount 0) | ForEach-Object { Redact-Line $_ }) -Encoding utf8

    $subEnd = Get-LastStepTime $src
    $age = if ($subEnd) { [int][Math]::Max(0, ($sessionEnd - $subEnd).TotalSeconds) } else { 0 }
    $ages[$id] = $age
}

# --- meta --------------------------------------------------------------------
@{
    conversation  = $convId
    expect        = $Expect
    note          = $Note
    captured      = (Get-Date).ToString('o')
    # Seconds between each subagent's last step and the end of the session. Re-applied as an
    # mtime at test time so the 90s liveness window means the same thing on every machine.
    subagent_ages = $ages
} | ConvertTo-Json -Depth 4 | Set-Content -Path (Join-Path $root 'meta.json') -Encoding utf8

Write-Host "Captured fixture: tests/fixtures/$Name"
Write-Host "  conversation : $convId"
Write-Host "  parent lines : $($lines.Count)"
Write-Host "  task logs    : $taskCount"
Write-Host "  subagents    : $($ages.Count)  $(($ages.Keys | ForEach-Object { "$_=$($ages[$_])s" }) -join ' ')"
Write-Host "  expect       : $Expect"
Write-Host "`nRun 'cargo test fixture' to check it."
