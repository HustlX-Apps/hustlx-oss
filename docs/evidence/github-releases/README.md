# GitHub release automation evidence

October 20, 2026. Implementation keeps the source version at 0.1.0. No release tag, draft or public GitHub Release is created during this task.

The change adds patch/minor/major and exact-version previews; an explicit GitHub build/publish command; native optimized Windows x64, macOS Apple Silicon and macOS Intel packages; asset validation; and publication after all jobs pass. Local Cargo packaging remains independent of GitHub.

Before implementation, new Rust tests failed because bump/preview commands were absent, and the Python suite failed because the release helper did not exist. Existing xtask baseline: 9 passing tests. Local post-change verification is recorded in the PR: Cargo workspace tests, Clippy, formatting, release safety tests, actionlint, version preview and package dry-run. GitHub interactions in Python tests are mocked; draft creation, uploading, publication and failure ordering are simulated without credentials or remote mutations.

PR CI builds real optimized packages and uploads Actions artifacts with publishing disabled. The final PR records each platform's actual result and the artifact verification job. Publication is intentionally not executed. Manual platform launch/offline/upgrade checks, dependency-notice review, official signing, notarization, installers and minimum-OS certification remain release work; see the [checklist](../../qa-checklists/desktop-release.md).
