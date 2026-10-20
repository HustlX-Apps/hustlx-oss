# Desktop versions and local packages

CI/CD is paused to save cost as of 2026-10-20. Repository GitHub Actions is disabled and hosted Rust/release workflows are removed. Run checks and native packaging locally. Do not dispatch hosted builds, enable Actions or restore automated publishing without an explicit user decision to resume CI/CD. See [the decision](notes.md).

`develop` remains the integration branch. Prepare versions on a feature branch through a develop PR. Local verification and the [manual release checklist](qa-checklists/desktop-release.md) remain required. No release is published by the checker or Cargo commands. Existing Python release helpers and [historical automation evidence](evidence/github-releases/README.md) are retained for future review, not as active release instructions.

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

Install the selected target with rustup before building. Cross-architecture SDK/linker availability remains your build-host responsibility. Cross-OS packaging and universal macOS binaries are not implemented. Each local check verifies its own native host; this list is script support, not hardware or minimum-OS certification.

## Output and signing

Each invocation creates a fresh `dist/hustlx-oss-VERSION-TARGET/` containing:

- macOS: `HustlX OSS.app` and a ZIP preserving the app bundle.
- Windows: a `HustlX OSS` portable directory with `hustlx-oss.exe`, and its ZIP.
- `SHA256SUMS` for the ZIP.

Packages include version/target metadata, the project MIT LICENSE, third-party notices and font licenses. The project license is required during planning and copied into macOS Contents/Resources or the Windows portable directory. They do not copy your local database, exports, credentials or arbitrary repository files. Existing output directories are never overwritten; move a previous or failed staging directory before retrying.

macOS bundles use `com.hustlx.oss.desktop`, numeric version components in Info.plist, Retina support and ad-hoc signing without credentials. The full SemVer appears in the archive name and VERSION.txt. These are not Developer ID signed or notarized releases. Windows executables are unsigned; version/target metadata is in VERSION.txt, not a Windows executable version resource. Installers, launcher icons, OS trust prompts, supported OS/CPU testing and official signing/notarization remain release work.

Before distributing an actual build, complete the real-app launch/offline/upgrade journey on each supported OS and architecture, collect required Cargo dependency notices, preserve a closed-library backup and review the exact package contents. This tooling does not make those release claims.

## Verification

Run `bash scripts/check.sh` on macOS or `./scripts/check.ps1` in Windows PowerShell. This covers formatting, Cargo workspace tests, Clippy, Python release safety tests, a native debug build and packaging preview. Python tests mock GitHub operations and never publish. See [development](development.md) and [original local tooling evidence](evidence/release-tooling/README.md).

Actual distribution requires native package builds, archive/checksum review, launch/offline/upgrade evidence for each intended OS and architecture, dependency-notice review, and explicit publication authorization. A local macOS check does not certify Windows or macOS Intel. Hosted CI/CD remains paused until the user explicitly resumes it.
