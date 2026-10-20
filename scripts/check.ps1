$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    # Local checks only. Never dispatch hosted builds or publish releases here.
    cargo +stable fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo +stable test --locked --workspace --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo +stable clippy --locked --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    python -m unittest discover -s scripts -p test_github_release.py -v
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo +stable build --locked
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $releaseVersion = cargo +stable xtask version
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo +stable xtask release --version $releaseVersion --dry-run
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
