param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$env:PYTHONUTF8 = '1'
$taskProject = (Resolve-Path "$PSScriptRoot/..").Path
$taskWorkspace = (Resolve-Path "$taskProject/..").Path
$taskPython = Join-Path $env:USERPROFILE '.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
if (Test-Path (Join-Path $taskWorkspace 'tools/python-packages/slint')) {
    $env:PYTHONPATH = Join-Path $taskWorkspace 'tools/python-packages'
}
if (!(Test-Path $taskPython)) { $taskPython = 'python' }
$taskArgs = @((Join-Path $PSScriptRoot 'simulate_ui.py'))
if ($SelfTest) { $taskArgs += '--self-test' }
& $taskPython @taskArgs
exit $LASTEXITCODE
