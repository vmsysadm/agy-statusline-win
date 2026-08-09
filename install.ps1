<#
.SYNOPSIS
    Installs custom statusline for Antigravity CLI on Windows 11.
#>
$ErrorActionPreference = "Stop"

$targetDir = Join-Path $env:USERPROFILE ".gemini\antigravity-cli"
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$scriptUrl = "https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/statusline.ps1"
$configUrl = "https://raw.githubusercontent.com/vmsysadm/agy-statusline-win/main/statusline_config.json"

$ps1Path = Join-Path $targetDir "statusline.ps1"
$cfgPath = Join-Path $targetDir "statusline_config.json"
$settingsPath = Join-Path $targetDir "settings.json"

Write-Host "Downloading statusline.ps1..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $scriptUrl -OutFile $ps1Path

Write-Host "Downloading statusline_config.json..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $configUrl -OutFile $cfgPath

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

$cmdStr = "pwsh.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $ps1Path"

$settings.statusLine | Add-Member -NotePropertyName "command" -NotePropertyValue $cmdStr -Force
$settings.statusLine | Add-Member -NotePropertyName "configPath" -NotePropertyValue $cfgPath -Force
$settings.statusLine | Add-Member -NotePropertyName "enabled" -NotePropertyValue $true -Force

$settings | ConvertTo-Json -Depth 10 | Set-Content $settingsPath -Encoding utf8

Write-Host "Installation complete! Custom statusline is active." -ForegroundColor Green

