param(
    [Parameter(Mandatory = $true)]
    [string]$Destination
)
$ErrorActionPreference = 'Stop'
$taskProject = Split-Path $PSScriptRoot -Parent
# Preserve relative paths used by the notices; this is not a full dependency bundle.
$taskNoticeFiles = @(
    'LICENSE.md',
    'NOTICE.md',
    'COMMERCIAL-LICENSING.md',
    'THIRD-PARTY-NOTICES.md',
    'INDEPENDENT-IMPLEMENTATION.md',
    'core/data/README-LICENSING.md',
    'core/ui/fonts/OFL.txt',
    'vendor/esp-mbedtls/LICENSE-MIT',
    'vendor/esp-mbedtls/LICENSE-APACHE'
)
foreach ($taskRelative in $taskNoticeFiles) {
    $taskSource = Join-Path $taskProject $taskRelative
    if (-not (Test-Path -LiteralPath $taskSource -PathType Leaf)) {
        throw "Required license notice is missing: $taskRelative"
    }
}
foreach ($taskRelative in $taskNoticeFiles) {
    $taskTarget = Join-Path $Destination $taskRelative
    New-Item -ItemType Directory -Path (Split-Path $taskTarget -Parent) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $taskProject $taskRelative) -Destination $taskTarget
}
'NOT FOR DISTRIBUTION: embedded Slint licensing is unresolved. See THIRD-PARTY-NOTICES.md. These notices do not grant missing permissions.' |
    Set-Content -LiteralPath (Join-Path $Destination 'DISTRIBUTION-BLOCKED.txt') -Encoding utf8
