<#
.SYNOPSIS
    Installs Rust-compiled custom statusline for Antigravity CLI on Windows.
#>
$ErrorActionPreference = "Stop"

$targetDir = Join-Path $env:USERPROFILE ".gemini\antigravity-cli"
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$binTarget = Join-Path $targetDir "agy-statusline.exe"
$localBin = Join-Path $PSScriptRoot "target\release\agy-statusline.exe"
$downloadUrl = "https://github.com/vmsysadm/agy-statusline-win/releases/download/v1.0.0/agy-statusline.exe"

$configSource = Join-Path $PSScriptRoot "statusline_config.json"
$configTarget = Join-Path $targetDir "statusline_config.json"
if (Test-Path $configSource) {
    Copy-Item -Path $configSource -Destination $configTarget -Force
}

function Get-StagingPath { param([string]$Destination) return "$Destination.new" }

function Install-BinarySafely {
    param([string]$Source, [string]$Destination)
    $stagingFile = Get-StagingPath $Destination
    if ($Source -ne $stagingFile) {
        Copy-Item -Path $Source -Destination $stagingFile -Force
    }

    if (-not (Test-Path $Destination)) {
        Move-Item -Path $stagingFile -Destination $Destination -Force
        return
    }

    # The running binary may hold a lock on $Destination, so swap it aside instead of overwriting.
    $backupFile = "$Destination.old"
    if (Test-Path $backupFile) {
        Remove-Item $backupFile -Force -ErrorAction SilentlyContinue
    }
    try {
        [System.IO.File]::Replace($stagingFile, $Destination, $backupFile, $true)
    } catch {
        # Fallback if atomic replace fails (e.g. a stale, still-locked backup)
        Move-Item -Path $Destination -Destination $backupFile -Force -ErrorAction SilentlyContinue
        Move-Item -Path $stagingFile -Destination $Destination -Force -ErrorAction SilentlyContinue
        if (Test-Path $stagingFile) {
            Remove-Item $stagingFile -Force -ErrorAction SilentlyContinue
            throw "Could not replace '$Destination' - the file is in use. Close Antigravity CLI and any terminals running the statusline, then re-run this installer."
        }
    }

    # Best effort: the handle on the previous binary is usually gone by now.
    Remove-Item $backupFile -Force -ErrorAction SilentlyContinue
}

if (Test-Path $localBin) {
    Write-Host "Installing compiled agy-statusline.exe from local build..." -ForegroundColor Cyan
    Install-BinarySafely -Source $localBin -Destination $binTarget
} elseif (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "Building agy-statusline with Cargo..." -ForegroundColor Cyan
    Push-Location $PSScriptRoot
    cargo build --release
    $buildExit = $LASTEXITCODE
    Pop-Location
    if ($buildExit -ne 0) {
        throw "cargo build --release failed with exit code $buildExit."
    }
    Install-BinarySafely -Source $localBin -Destination $binTarget
} else {
    Write-Host "Downloading agy-statusline.exe release binary..." -ForegroundColor Cyan
    # Download straight into the staging slot so a partial download can never land on $binTarget.
    $tempDownload = Get-StagingPath $binTarget
    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $tempDownload
    } catch {
        Remove-Item $tempDownload -Force -ErrorAction SilentlyContinue
        throw
    }
    Install-BinarySafely -Source $tempDownload -Destination $binTarget
}

$settingsPath = Join-Path $targetDir "settings.json"
if (Test-Path $settingsPath) {
    Write-Host "Updating settings.json..." -ForegroundColor Cyan
    $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json
} else {
    Write-Host "Creating settings.json..." -ForegroundColor Cyan
    $settings = [PSCustomObject]@{
        colorScheme = "dark"
        enableTelemetry = $false
    }
}

if (-not $settings.statusLine) {
    $settings | Add-Member -NotePropertyName "statusLine" -NotePropertyValue ([PSCustomObject]@{}) -Force
}

$settings.statusLine | Add-Member -NotePropertyName "command" -NotePropertyValue $binTarget -Force
$settings.statusLine | Add-Member -NotePropertyName "enabled" -NotePropertyValue $true -Force

$json = $settings | ConvertTo-Json -Depth 10
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($settingsPath, $json, $utf8NoBom)

Write-Host "Installation complete! High-performance Rust statusline is active." -ForegroundColor Green
