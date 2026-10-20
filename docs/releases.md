# Desktop versions, builds and GitHub Releases

Use local Cargo commands to prepare versions and packages, and the separate GitHub command to build all release targets or explicitly publish. Publishing is disabled by default. Installing this automation, opening a PR, merging a PR, or pushing a tag does not publish a release. No release was published while implementing this tooling. The app remains a development build; project code and documentation use the [MIT License](../LICENSE).

## Branches

`develop` is the default integration branch. Prepare versions on a clean feature branch through a PR targeting develop. Promote reviewed release changes from develop to main through a separate PR. Publishing requires the exact main commit and matching Cargo version. The workflow does not merge PRs or push protected branches. Merge the release workflow into the default branch before manual dispatch becomes available; promote it to main before publishing from main.

## Version command

Run from the repository root with Rust installed:

```sh
cargo xtask version
cargo xtask version --bump patch --dry-run
cargo xtask version --bump patch
cargo xtask version 0.2.0-beta.1 --dry-run
cargo xtask version 0.2.0-beta.1
```

`--bump patch|minor|major` computes the next version and resets lower components. `--dry-run` prints the proposed version without changing files. An exact version supports prereleases; automatic bumps reject prerelease/build versions to avoid guessing their graduation policy. A version write updates only the app entries in Cargo.toml and Cargo.lock, preserving dependency and xtask versions. Review and commit that diff on a feature branch through a PR. Keep version preparation separate from other Cargo operations. If a write is interrupted, restore both version files from Git; packaging refuses mismatched versions.

Versions use SemVer without a `v` prefix, such as `0.2.0` or `0.2.0-beta.1`. Git tags use `vVERSION`. Implementation of this automation leaves the source version at 0.1.0.

## Build all desktop packages on GitHub

Install Python 3.9+ and the authenticated GitHub CLI. Run these commands from this repository. Bash and PowerShell both support this command syntax; Windows may use `python` instead of `python3`.

Obtain the exact remote branch commit using `git fetch origin` and `git rev-parse origin/develop`. Substitute that full 40-character SHA for FULL_SHA, and use the Cargo version at that commit:

```sh
python3 scripts/github_release.py dispatch --version 0.1.0 --sha FULL_SHA --ref develop --dry-run
python3 scripts/github_release.py dispatch --version 0.1.0 --sha FULL_SHA --ref develop
gh run list --repo HustlX-Apps/hustlx-oss --workflow release.yml
gh run watch RUN_ID --repo HustlX-Apps/hustlx-oss --exit-status
gh run download RUN_ID --repo HustlX-Apps/hustlx-oss --pattern 'desktop-*' --dir artifacts
python3 scripts/github_release.py collect --version 0.1.0
```

The first dispatch command prints the plan and makes no GitHub request. The second starts a build with `publish=false`, producing Actions artifacts only. Relevant source/tooling PRs also run this lane without publishing. Native runners test, lint, compile optimized applications, and package macOS Apple Silicon, macOS Intel and Windows x64 from the same exact commit. A final job checks every ZIP and checksum. Build artifacts expire after seven days. `collect` creates a fresh release-assets directory containing three ZIPs and an aggregate SHA256SUMS; preserve or move an existing destination before retrying.

## Publish when a release is authorized

Complete the [release checklist](qa-checklists/desktop-release.md), merge the version PR to develop, and promote the reviewed release to main. Fetch and obtain `git rev-parse origin/main`; use that exact SHA and matching version:

```sh
python3 scripts/github_release.py dispatch --version 0.1.0 --sha FULL_MAIN_SHA --ref main --publish --dry-run
python3 scripts/github_release.py dispatch --version 0.1.0 --sha FULL_MAIN_SHA --ref main --publish
```

Only the second command authorizes remote publication. GitHub's workflow-dispatch UI has the same version, source_sha and publish inputs; publish defaults to false. Validation rejects wrong source/version, publication from develop and existing tags. The publish job requires a manual publish=true dispatch on main in this repository, after every native build and asset check succeeds. Only that job receives contents:write; build jobs have read-only permissions and do not persist checkout credentials. GitHub provides the job token; no personal token or signing credentials are required.

Publication creates an immutable-source `vVERSION` tag, generates notes with source/signing disclosures, creates a draft, uploads three ZIPs plus aggregate SHA256SUMS, then publishes. Versions such as `0.2.0-beta.1` create prereleases and never become Latest. This publishes downloadable desktop packages to GitHub Releases, not crates.io or an app store. Installers, official signing and notarization remain separate work.

### Failure and recovery

A failed build or asset check creates no tag or release. A failure after tag creation may leave a tag and draft. Upload failure leaves the draft unpublished; an ambiguous final network error requires inspection because the server may have completed publication. The workflow refuses existing exact tags, never moves them, and never clobbers assets. API/permission errors stop the run instead of being treated as absence.

Inspect `gh run view RUN_ID --repo HustlX-Apps/hustlx-oss --log-failed` and `gh release view vVERSION --repo HustlX-Apps/hustlx-oss`. Preserve the tag, draft and artifacts. A maintainer must choose a new version or explicitly authorize recovery of the exact draft after checking its source, digests and missing uploads. Do not delete or force-push a public tag as a retry shortcut.

## Agent skill and command

In the umbrella workspace, `/oss-release prepare patch`, `/oss-release build VERSION`, `/oss-release publish VERSION` and `/oss-release status` load the optional canonical `.agents/skill-library/hustlx-oss-release/SKILL.md`. AgentSync exposes the skill/command to Claude and Cursor. Codex can explicitly load that library path and request a mode without expanding its default catalog. Preparation handles the version branch, verification and PR; it does not publish or auto-merge. Rust/Python commands also work in a standalone clone.

## Preview the build plan

```sh
cargo xtask release --version 0.1.0 --dry-run
```

This validates the version, supported platform/target, required notices and fresh destination, then prints the planned archive. It does not build the application or create distribution files. The small xtask command itself is compiled by Cargo as needed.

## Build a local package when ready

Use the same command on the target operating system:

```sh
cargo xtask release --version 0.1.0
```

Equivalent macOS wrapper:

```sh
./scripts/release.sh --version 0.1.0
./scripts/release.sh --version 0.1.0 --dry-run
```

Equivalent Windows PowerShell wrapper:

```powershell
.\scripts\release.ps1 -Version 0.1.0
.\scripts\release.ps1 -Version 0.1.0 -DryRun
```

Use the checked-in scripts according to your existing PowerShell policy. Rust, the native platform build tools and the target's standard library must be installed. macOS requires Xcode Command Line Tools; Windows requires the MSVC toolchain, C++ build tools and Windows SDK. First builds need Cargo dependencies downloaded. SQLite, fonts and artwork are bundled in the executable.

The version argument must exactly match Cargo.toml and Cargo.lock. The tool builds with `cargo build --locked --release --package hustlx-oss --target TARGET --target-dir target`. The default target is the Rust host. `--target` accepts these targets only on the matching OS:

| Host OS | Targets accepted by the script |
| --- | --- |
| macOS | `aarch64-apple-darwin`, `x86_64-apple-darwin` |
| Windows | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` |

Install the selected target with rustup before building. Cross-architecture SDK/linker availability remains your build-host responsibility. Cross-OS packaging and universal macOS binaries are not implemented. Current CI verifies its own native host; this list is script support, not hardware or minimum-OS certification.

## Output and signing

Each invocation creates a fresh `dist/hustlx-oss-VERSION-TARGET/` containing:

- macOS: `HustlX OSS.app` and a ZIP preserving the app bundle.
- Windows: a `HustlX OSS` portable directory with `hustlx-oss.exe`, and its ZIP.
- `SHA256SUMS` for the ZIP.

Packages include version/target metadata, the project MIT LICENSE, third-party notices and font licenses. The project license is required during planning and copied into macOS Contents/Resources or the Windows portable directory. They do not copy your local database, exports, credentials or arbitrary repository files. Existing output directories are never overwritten; move a previous or failed staging directory before retrying.

macOS bundles use `com.hustlx.oss.desktop`, numeric version components in Info.plist, Retina support and ad-hoc signing without credentials. The full SemVer appears in the archive name and VERSION.txt. These are not Developer ID signed or notarized releases. Windows executables are unsigned; version/target metadata is in VERSION.txt, not a Windows executable version resource. Installers, launcher icons, OS trust prompts, supported OS/CPU testing and official signing/notarization remain release work.

Before distributing an actual build, complete the real-app launch/offline/upgrade journey on each supported OS and architecture, collect required Cargo dependency notices, preserve a closed-library backup and review the exact package contents. This tooling does not make those release claims.

## Verification

```sh
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Also run `python3 -m unittest discover -s scripts -p test_github_release.py -v` and `actionlint .github/workflows/release.yml .github/workflows/rust.yml` when changing release automation. Tests cover version preview/bumping, platform package fixtures, tampered/missing artifacts, exact-source validation, publish authorization, existing tags, draft upload failure, and prerelease behavior. GitHub mutations are mocked in unit tests. `rust.yml` dry-runs native wrappers; the separate release workflow builds and uploads real optimized packages. macOS fixture tests validate plist/ad-hoc signatures. Windows checksums use .NET SHA256 without module auto-loading. See [original local tooling evidence](evidence/release-tooling/README.md) and [automation evidence](evidence/github-releases/README.md) for the tested scope.
