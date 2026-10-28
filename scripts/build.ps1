param(
    [switch]$Mirror,
    [switch]$Baseline
)
$ErrorActionPreference = 'Stop'
$taskWorkspace = (Resolve-Path "$PSScriptRoot/../..").Path
$taskProject = (Resolve-Path "$PSScriptRoot/..").Path
if ($Baseline) {
    $taskProject = Join-Path $taskWorkspace 'baseline/SpoolEase-3532f8d962dd1a95c7d4ebb37beddca5bbefd39a'
}
$env:CARGO_HOME = Join-Path $taskWorkspace 'tools/cargo'
$env:RUSTUP_HOME = Join-Path $taskWorkspace 'tools/rustup'
$env:PATH = "$env:CARGO_HOME/bin;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;$env:PATH"
. (Join-Path $taskWorkspace 'tools/export-esp.ps1')
# V2 owns its build output; the V1 source and firmware remain independent.
$env:CARGO_TARGET_DIR = Join-Path $taskProject 'target'
$taskArgs = @('+esp1')
if ($Mirror) {
    $taskArgs += @('--config', 'source.crates-io.replace-with="rsproxy"', '--config', 'source.rsproxy.registry="sparse+https://rsproxy.cn/index/"')
}
$taskArgs += @('build', '--release', '--locked')
Push-Location (Join-Path $taskProject 'core')
try {
    & "$env:CARGO_HOME/bin/cargo.exe" @taskArgs
    $taskExit = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $taskExit

