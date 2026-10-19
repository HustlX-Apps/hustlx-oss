[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Target,
    [switch]$DryRun
)
$ErrorActionPreference = 'Stop'
$releaseArgs = @('xtask', 'release', '--version', $Version)
if ($Target) { $releaseArgs += @('--target', $Target) }
if ($DryRun) { $releaseArgs += '--dry-run' }
Push-Location -LiteralPath (Join-Path $PSScriptRoot '..')
try {
    & cargo @releaseArgs
    $releaseExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $releaseExitCode
