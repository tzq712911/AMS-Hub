$ErrorActionPreference = 'Stop'
$taskWorkspace = (Resolve-Path "$PSScriptRoot/../..").Path
$taskProject = (Resolve-Path "$PSScriptRoot/..").Path
$taskElf = Join-Path $taskWorkspace 'AMS-Hub/target/xtensa-esp32s3-none-elf/release/SpoolEase'
$taskDist = Join-Path $taskProject 'dist'
New-Item -ItemType Directory -Path $taskDist -Force | Out-Null
$taskApp = Join-Path $taskDist 'AMS-Hub-0.6.0-alpha.3-app.bin'
& (Join-Path $taskWorkspace 'tools/espflash/espflash.exe') save-image --chip esp32s3 --flash-size 16mb --flash-mode dio --flash-freq 80mhz --partition-table (Join-Path $taskProject 'core/partitions.csv') $taskElf $taskApp
if ($LASTEXITCODE -ne 0) { throw 'Application image generation failed' }
Copy-Item -LiteralPath (Join-Path $taskProject 'core/partitions.csv') -Destination (Join-Path $taskDist 'partitions.csv')
$taskHash = Get-FileHash -LiteralPath $taskApp -Algorithm SHA256
@{
    customVersion = '0.6.0-alpha.3+ams-hub'
    upstreamCommit = '3532f8d962dd1a95c7d4ebb37beddca5bbefd39a'
    chip = 'esp32s3'
    hardware = 'WT32-SC01 Plus + PN532 SPI + microSD'
    imageKind = 'application-only; not a merged full-flash image'
    flashBytes = 16777216
    applicationPartitionBytes = 7340032
    imageBytes = (Get-Item -LiteralPath $taskApp).Length
    sha256 = $taskHash.Hash
    hardwareTested = $false
    buildOnly = $true
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskDist 'build-info.json') -Encoding utf8
Write-Output "Application image: $taskApp"
