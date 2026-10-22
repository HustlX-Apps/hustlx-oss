# Sport setup and form refinement

The requested port builds on OSS develop `8bd5c0d`, which already contains all nine sport consoles. Nuxt reference `b3a9a2a8d` adds hosted league statistics and console refinements; league management remains outside the standalone match application. Reuse native scoring and durable local commands rather than copying private Vue source.

Approved by the user on October 22, 2026.

## Design

Preserve the native Exo/Inter typography, existing light/dark palette and navigation. Taste settings: design variance 3, motion 1, density 5. This is an operator workspace: predictable grouping and legible controls take priority over decorative treatments.

Use a wider creation panel with match details, participants and sport rules clearly separated. Place related short fields side by side. Keep Create match outside the scrolling form. Keep the saved-match list independently scrollable. Show persistent labels, contextual help and validation beside relevant fields. Provide optional roster removal, retain draft names when changing sports, and distinguish sets, games, pairs and regulation periods. Team/player editors use the same label and feedback rhythm.

A cosmetic spacing-only pass would leave submission errors and roster-reset friction. A full wizard would add navigation to a frequent operation. The selected single-page grouped form preserves the existing workflow and makes its next action visible.

## Work checklist

- [x] Compare fresh OSS and Nuxt develop; identify existing nine-sport port.
- [x] Record baseline and prepare HTML form preview.
- [x] Add behavioral regressions for invalid setup, corrected input and optional rosters.
- [x] Refine native match/sport/team/player forms without persistence changes.
- [x] Inspect both themes at minimum window size and create/reopen sport matches.
- [x] Run local checker and sport harness, document evidence and complete the local merge-safety review.

Runtime truth: OSS App/Session/Store and scoring engines. Execution: local Rust and native UI tooling. Verification: Cargo regressions, `bash scripts/check.sh`, sport harness and native screenshots. Documentation: OSS operator guide and evidence. Hosted launch and payment boundaries are unaffected.
