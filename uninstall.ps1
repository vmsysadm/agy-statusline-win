<#
.SYNOPSIS
    Uninstalls custom statusline for Antigravity CLI on Windows 11.
#>
$targetDir = Join-Path $env:USERPROFILE ".gemini\antigravity-cli"
$settingsPath = Join-Path $targetDir "settings.json"

if (Test-Path $settingsPath) {
    $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json
    if ($settings.statusLine) {
        $settings.statusLine.enabled = $false
        $settings | ConvertTo-Json -Depth 10 | Set-Content $settingsPath -Encoding utf8
        Write-Host "Disabled custom statusline in settings.json." -ForegroundColor Yellow
    }
}
