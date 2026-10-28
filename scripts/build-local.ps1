param([switch]$Mirror)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskCargo = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
$taskExport = Join-Path $env:USERPROFILE 'export-esp.ps1'
if (-not (Test-Path $taskCargo)) { throw 'Rustup is not installed.' }
if (-not (Test-Path $taskExport)) { throw 'Run espup install --name esp1 --targets esp32s3 --toolchain-version 1.90.0 first.' }
. $taskExport
$env:PATH = "$(Split-Path $taskCargo);$env:PATH"
$taskVsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (-not (Test-Path $taskVsWhere)) { throw 'Visual Studio C++ Build Tools are required.' }
$taskVs = & $taskVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $taskVs) { throw 'Visual Studio C++ workload is missing or installation is incomplete.' }
$taskCmake = Join-Path $taskVs 'Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin'
if (Test-Path $taskCmake) { $env:PATH = "$taskCmake;$env:PATH" }
$env:CARGO_TARGET_DIR = Join-Path $taskRoot 'target'
$taskArgs = @('+esp1')
if ($Mirror) {
    $taskArgs += @('--config', 'source.crates-io.replace-with="rsproxy"', '--config', 'source.rsproxy.registry="sparse+https://rsproxy.cn/index/"')
}
$taskArgs += @('build', '--release', '--locked')
Push-Location (Join-Path $taskRoot 'core')
try {
    & $taskCargo @taskArgs
    if ($LASTEXITCODE -ne 0) { throw "Build failed with exit code $LASTEXITCODE" }
} finally { Pop-Location }
