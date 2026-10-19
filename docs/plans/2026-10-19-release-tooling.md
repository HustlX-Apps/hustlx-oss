# Versioned desktop packaging implementation plan

Goal: provide commands to prepare a version and build native macOS/Windows packages without tagging, pushing or publishing a release.

Repo set: hustlx-oss. Source of truth: Cargo.toml package version and Cargo.lock. Execution: Rust xtask with thin Bash/PowerShell wrappers; GitHub CLI for branch/PR metadata. Verification: isolated version/parser/package/archive tests on both CI hosts, formatter, Clippy, dry-run, existing application tests through CI, shared verifier boundary and merge flagger. Documentation impact: local release/development guidance and roadmap. Payments, hosted contracts and runtime game behavior are unaffected.

Design: reuse PR #1 to develop. Create main from existing develop through a new Git ref, without merging unreviewed runtime changes. Keep develop as default. Add a zero-dependency Rust workspace tool invoked by cargo xtask. Separate version preparation from packaging. Explicit package version must match the manifest/lockfile. Build for one supported native-platform target at a time; macOS aarch64/x86_64 and Windows MSVC x86_64/aarch64. CI verifies its current host target; other architecture/toolchain compatibility remains a release check.

macOS output is an ad-hoc signed .app ZIP with versioned Info.plist. Windows output is an unsigned portable EXE ZIP. Both include version/target metadata, third-party/font notices and SHA-256 checksums. Refuse overwriting an output directory. No installers, Developer ID signing, notarization, release tags, uploads, GitHub Release automation or crates.io publishing. Dry-run creates no distribution files and does not build the app. Only temporary synthetic package fixtures are built while implementing this request.

- [x] Write failing tests for strict version syntax, package selection, manifest/lock updates and packaging boundaries.
- [x] Implement version preparation, guarded release planning, native packaging/checksum commands and wrappers.
- [x] Add workspace-tool verification to Windows/macOS CI; document exact commands and release limits.
- [x] Run local tooling checks and dry-run only, update/push existing PR, assess final CI. Do not run a real release build or publish.
