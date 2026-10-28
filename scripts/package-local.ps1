$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskFlash = Join-Path $taskRoot '.local-tools/espflash/espflash.exe'
$taskElf = Join-Path $taskRoot 'target/xtensa-esp32s3-none-elf/release/SpoolEase'
$taskPartitions = Join-Path $taskRoot 'core/partitions.csv'
if (-not (Test-Path $taskElf)) { throw 'Build the firmware with scripts/build-local.ps1 first.' }
if (-not (Test-Path $taskFlash)) { throw 'espflash 4.6.0 is missing from .local-tools/espflash.' }
$taskDist = Join-Path $taskRoot 'dist/local'
New-Item -ItemType Directory -Path $taskDist -Force | Out-Null
$taskImage = Join-Path $taskDist 'AMS-Hub-full.bin'
& $taskFlash save-image --chip esp32s3 --flash-size 16mb --flash-mode dio --flash-freq 80mhz --partition-table $taskPartitions --target-app-partition ota_0 --merge --skip-update-check $taskElf $taskImage
if ($LASTEXITCODE -ne 0) { throw 'Failed to generate full flash image.' }
Copy-Item -LiteralPath $taskPartitions -Destination (Join-Path $taskDist 'partitions.csv')
$taskHash = (Get-FileHash -LiteralPath $taskImage -Algorithm SHA256).Hash.ToLowerInvariant()
"$taskHash  AMS-Hub-full.bin" | Set-Content -LiteralPath (Join-Path $taskDist 'SHA256SUMS.txt') -Encoding ascii
Write-Output "Full image for a blank WT32-SC01 Plus: $taskImage"
Write-Output 'Flash address: 0x0. Hardware testing has not been performed.'
