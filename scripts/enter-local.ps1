# Dot-source this file from PowerShell to enable the local ESP32-S3 tools.
$taskRoot = Split-Path $PSScriptRoot -Parent
$env:PATH = "$(Join-Path $env:USERPROFILE '.cargo/bin');$(Join-Path $taskRoot '.local-tools/espflash');$(Join-Path $taskRoot '.local-tools');$env:PATH"
. (Join-Path $env:USERPROFILE 'export-esp.ps1')
Write-Output 'AMS-Hub tools enabled: cargo +esp1, espup, espflash.'
