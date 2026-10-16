# HustlX branding implementation plan

**Goal:** Reuse the existing HustlX identity in the Rust desktop app: official wordmarks, Inter body/control text, Exo headings and Exo Black scores/names, with native light/dark brand palettes.

**Architecture:** Bundle existing vector wordmarks and licensed native TTF files in the executable. Centralize fonts, palettes, and logo selection in a Rust branding module. Keep SQLite and game behavior unchanged. Use the approved web v2 layout.

**Tech stack:** Existing Rust/Iced application, Iced SVG support, static Inter and Exo TTF assets.

## Scope and source

- Runtime edits: hustlx-oss only, continuing codex/rust-native-preview and PR #1 targeting develop.
- Reference: hustlx-nuxt4 tailwind.config.ts, app/assets/css/tokens.css, and public/hustlx-wordmark-{white,navy}.svg. Static TTF assets from native HustlX Android/iOS, with their OFL notices.
- Execution: local terminal and native app controls. Verification: Cargo formatter/tests/Clippy/build, shared verifier boundary, merge flagger, Windows/macOS CI, and rendered minimum-size screenshots.
- Documentation: asset provenance/notices, branding guidance, current screenshots and verification evidence in this repository. No umbrella or customer-doc edits.

## Implementation

- [ ] Copy official SVGs and native Inter Regular/SemiBold/Bold, Exo Bold/Black plus licenses; remove unused Fira Sans assets.
- [ ] Register embedded fonts by their actual internal family names and weights. Use Inter for controls/body, Exo for headings, Exo Black for scores and entity names.
- [ ] Add theme-aware wordmark plus compact OSS label to the navigation. Map control and page colors to HustlX light/navy/gold and dark/graphite/indigo tokens. Retain semantic stat colors.
- [ ] Render Matches, Teams/Players, and saved Game Day at 1100x760 in both themes. Capture screenshots, inspect glyphs/weights, contrast, wrapping, and focusable forms.
- [ ] Update asset provenance and docs. Run Cargo gates, required workspace verifier boundary, and merge flagger. Push PR update and assess Windows/macOS CI.

This is a reversible presentation/asset change. No implementation-mirroring UI tests are needed; native renders are required evidence. No network font loading, game-rule changes, or new sync scope.
