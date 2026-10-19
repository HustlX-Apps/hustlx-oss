# Release-tooling evidence

Prepared October 19, 2026. No application version bump, release tag, GitHub Release, upload or real release package was created.

- Created remote/local main from develop commit `4ee157c5d88d011866706d45e5b7ab5af7c6a1e5`. The existing implementation PR #1 continues to target develop; no protected branch was merged or changed with implementation commits.
- New tests failed against the missing tool before implementation.
- `cargo test --locked --workspace --all-targets`: 52 passed on macOS (44 app tests and 8 tooling tests). Windows runs 51 because the signing fixture is macOS-only.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo build --locked`: passed. This is the existing debug app build, not a release artifact.
- `cargo fmt --all -- --check`, `bash -n scripts/release.sh`, whitespace and local doc links: checked before handoff.
- `cargo xtask release --version 0.1.0 --dry-run` and the Bash wrapper with the same arguments: passed. Neither created a repository dist directory or built the release executable.
- Temporary tests verify SemVer and invalid inputs, numeric bundle versions, app-only version changes, dependency/tool version preservation, manifest/lock mismatch rejection without edits, target/OS mismatch rejection, no mutation from planning, output overwrite refusal, explicit package allowlist, copied notices, archive extraction and checksum output. A tiny synthetic Rust app proves generated macOS plist validity and ad-hoc signing. No actual HustlX app is packaged by these tests.
- Windows/macOS CI runs workspace checks and dry-runs each native wrapper. The final PR records its current-head CI result. Windows archive/checksum tests use built-in PowerShell; the PowerShell wrapper cannot run on the local macOS host because PowerShell is not installed.

The shared umbrella verifier has no Rust lane. Cargo/CI own this evidence. No hosted API, payment, subscription, game data or runtime match behavior changed. No root docs were edited.

Actual optimized app packaging, ARM Windows and Intel macOS target builds, hardware launch tests, installers, icons, Developer ID/notarization, Windows code signing and minimum OS certification remain release checks. At the time of this evidence, the project license was undecided; MIT was subsequently adopted on October 19, 2026 (see [LICENSE](../../../LICENSE)). The existing upstream block 0.1.6 future-compatibility warning remains.

The first Windows CI found that the child Windows PowerShell process could not resolve Get-FileHash after creating its ZIP. Checksum generation now uses .NET SHA256 directly, avoiding cmdlet/module auto-loading. A known SHA-256 digest test covers this path with spaces and an apostrophe in the filename, in addition to ZIP extraction/checksum tests.
