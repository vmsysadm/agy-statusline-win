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
$downloadUrl = "https://github.com/vmsysadm/agy-statusline-win/releases/download/v0.2.0/agy-statusline.exe"

$configSource = Join-Path $PSScriptRoot "statusline_config.json"
$configTarget = Join-Path $targetDir "statusline_config.json"
if (Test-Path $configSource) {
    Copy-Item -Path $configSource -Destination $configTarget -Force
}

if (Test-Path $localBin) {
    Write-Host "Installing compiled agy-statusline.exe from local build..." -ForegroundColor Cyan
    Copy-Item -Path $localBin -Destination $binTarget -Force
} elseif (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "Building agy-statusline with Cargo..." -ForegroundColor Cyan
    Push-Location $PSScriptRoot
    cargo build --release
    Pop-Location
    Copy-Item -Path $localBin -Destination $binTarget -Force
} else {
    Write-Host "Downloading agy-statusline.exe release binary..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $downloadUrl -OutFile $binTarget
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
