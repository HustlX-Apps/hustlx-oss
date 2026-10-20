# Desktop release checklist

Record evidence for the exact source commit and version. Building packages does not complete the manual gates below.

- [ ] The version change was reviewed through a develop PR and the release was promoted through a main PR. Cargo.toml and Cargo.lock match; the target tag does not exist.
- [ ] The source version and full SHA match the requested release. Review changes and known limitations since the preceding release.
- [ ] Formatting, workspace tests, Clippy, release safety tests and actionlint pass. The native release matrix produces all three actual optimized packages and checksum verification passes.
- [ ] On macOS Apple Silicon, macOS Intel and Windows x64, download/extract the exact candidate ZIP, verify its checksum and launch the app. Record the OS/CPU and any trust prompt. A build runner does not prove a supported minimum OS or a graphical launch.
- [ ] With a disposable library, create teams/players and a match, record/undo statistics, operate clocks and lineups, quit/restart, and confirm history and paused recovery while offline. Follow development.md's native smoke checks.
- [ ] Close the previous app, back up its library, and verify upgrade/reopen with the candidate while preserving history. Do not experiment on an operator's only library.
- [ ] Inspect archive contents: expected app/EXE, metadata, project MIT license, required dependency/font notices and no local data or credentials. Review dependency licensing before actual distribution.
- [ ] Release notes name the version/source, actual assets, development limits, unsigned Windows status and ad-hoc/non-notarized macOS status. Installers and store distribution are not claimed.
- [ ] Publication is explicitly authorized. Run the build-only path for package review; invoke --publish only for the intended main commit. A prerelease must retain its prerelease designation.
- [ ] After publication, inspect the GitHub Release tag/source, public/prerelease status, three ZIPs and aggregate SHA256SUMS. Download and verify checksums. Retain the run link and manual evidence.

Negative-path regression coverage is in scripts/test_github_release.py: wrong source/version, non-main/PR/fork publication, missing explicit publish, existing tags, missing/extra/tampered assets, upload failure and prerelease Latest behavior. Version preview/bumping and manifest/lock mismatch guards are covered by xtask tests. Unit tests simulate GitHub operations; they never publish.
