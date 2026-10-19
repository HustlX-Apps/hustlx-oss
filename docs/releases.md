# Version preparation and local desktop packages

These commands prepare source versions and build local packages. They never create tags, push branches, upload artifacts, publish GitHub Releases or publish to crates.io. No release has been published. The app remains a development build; project code and documentation use the [MIT License](../LICENSE), and release-readiness checks are still pending.

## Branches

`develop` is the default integration branch. PR #1 carries the initial desktop implementation and tooling into it. `main` was created from the existing develop baseline, without merging that implementation. Promote reviewed stable changes from develop to main through a separate PR later. Packaging does not merge or change branches.

## Version command

Run from the repository root with Rust installed:

```sh
cargo xtask version
cargo xtask version 0.2.0
```

The first command prints the current app version. The second updates only the `hustlx-oss` package version in Cargo.toml and Cargo.lock. It does not change dependency versions or the xtask tool version. Review and commit that diff on a feature branch through a PR before a real release. Keep version preparation separate from other Cargo operations. If a write is interrupted, restore both version files from Git; packaging refuses mismatched versions.

Versions use SemVer without a `v` prefix, such as `0.2.0` or `0.2.0-beta.1`. The repository remains at 0.1.0 until a maintainer prepares another version.

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

Tests build archives from temporary synthetic fixtures and verify extraction and checksum generation. macOS additionally compiles a tiny fixture app and validates its plist and ad-hoc signature. Windows checksums use .NET SHA256 directly inside PowerShell so inherited module paths do not require Get-FileHash. CI dry-runs both OS wrappers. It does not build/upload actual release packages. [Implementation evidence](evidence/release-tooling/README.md) records the tested scope.
