# Native Rust preview evidence

Recorded October 16, 2026 on macOS with Rust 1.99.1 and Iced 0.14.0. This is the native Rust executable, separate from the original web reference screenshots.

- [Dark capture](dark.jpg)
- [Light capture](light.jpg)

Both captures show the minimum/default 1100 × 760 content area at 2× display scale, plus the native title bar. The preview was also inspected at 1440 × 900 before adopting the smaller initial window. Roster and feed scrolling preserve access to content in the compact window.

## Automated results

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked --all-targets`: passed, six behavioral tests.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo build --locked`: passed.
- `cargo test --locked --no-default-features --lib --test preview`: passed, six tests without the renderer.

Before domain implementation, five behavioral tests failed on missing scoring/recording behavior; the unknown-identity rejection test already passed against the empty model. After implementation, all six passed. Cases cover selected-player attribution, home/away isolation, made/attempted shooting counters, selection clearing, undo, unknown/wrong-side IDs, rebounds, other stats, and discipline counters.

The build emits an upstream future-compatibility warning for `block 0.1.6`, reached through Iced's Metal/wgpu dependency chain. Current builds and Clippy pass. Track upstream compatibility before a release; no dependency lint has been suppressed.

## Native checks

The native window was opened and exercised directly. A selected home player's made three increased his points and the home score by three, cleared selection, and created one feed row. Undo restored the score. An away player's missed free throw created a feed row without increasing points. Switching themes preserved the current selection and match. Closing and reopening started a fresh sample, as the persistent unsaved notice promises. The final compact captures show a home made three and an away player selected.

The UI uses custom-rendered widgets. Native accessibility-tree coverage and full keyboard operation have not been established and require a separate review before release.

## Shared verifier boundary

`bash .agents/scripts/verify/verify-changed.sh --strict` was run from the umbrella workspace. It exited 1 because unrelated existing `hustlx-api/public` generated assets have no mapped targeted tests. The runner maps six existing repositories and does not include `hustlx-oss` or a Rust lane. This is not a passing verifier result for this PR; the owning Cargo results above provide this preview's verification. No unrelated API files were changed for this task.

## Remaining release evidence

Windows build/test CI and native Windows interaction checks must be assessed separately. A CI configuration alone does not prove a passing run. Installer/signing, durable storage, recovery, actual clocks, exports, and full match workflows are outside this preview. At the time of this preview, the project code license was pending; MIT was subsequently adopted on October 19, 2026 (see [LICENSE](../../../LICENSE)).
