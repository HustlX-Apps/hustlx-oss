# GitHub release automation implementation plan

Goal: prepare SemVer versions, build Windows x64 and macOS Apple Silicon/Intel packages, and publish GitHub Releases only after an explicit publishing request. Do not bump this checkout, create a tag, or publish a release during implementation.

Design: retain Cargo.toml/Cargo.lock and the native Rust packager as the source of truth. Add patch/minor/major version preparation with a preview. Use a manually dispatched GitHub workflow with publishing disabled by default and a PR build lane. Pin all packages to the same commit; publishing requires that exact commit on main, successful native checks, all three archives, and valid checksums. Create a draft, upload the complete asset set, and expose it only after upload succeeds. Existing tags/releases are never overwritten. Prerelease SemVer creates a GitHub prerelease.

Alternatives: release-on-every-merge would publish unfinished development work; a third-party release manager would duplicate the existing version/packaging tool. Explicit version preparation plus native GitHub Actions fits the repository and keeps publishing a separate decision.

Repo set: hustlx-oss and umbrella .agents/docs. Execution: Rust xtask, Python standard library, GitHub Actions/CLI. Verification: version tests, isolated asset fixtures and mocked GitHub mutations, Cargo workspace tests/Clippy/format, actionlint, native PR checks, skill validation, AgentSync, shared verifier and merge flagger. Documentation: local release guide, shared workflow routing and skill registry. No basketball or persistence behavior changes.

- [x] Add failing version bump/preview and release safety tests, then implement the commands.
- [x] Add native build/artifact workflow and gated publishing job; run PR package verification without publishing.
- [x] Add the shared release skill and slash command through canonical AgentSync configuration.
- [x] Update guides and evidence, run required checks, and open PRs targeting develop. No release dispatch with publishing enabled, tags, or GitHub Release creation.

Runtime PR #3 records the native build/artifact results. Shared skill/command: HustlX-Apps/apps PR #762. Release publication and manual platform certification are intentionally outside this implementation handoff.
