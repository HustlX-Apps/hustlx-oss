# Teams layout evidence

Recorded October 21, 2026 on macOS arm64. Native Rust/Iced app at its minimum 1100x760 content size. Captures include the title bar at 2x. All records are synthetic in `/tmp/hustlx-oss-teams-review`; the normal operator library was not opened.

The Teams screen has a 340px sidebar, full-width team rows, a selected-team state, neutral secondary actions and labeled roster fields. Creation controls and roster fields remain visible while team/member lists scroll independently. Jersey labels use a contained, wrapping badge so 12-character labels do not overlap names or actions. No schema, storage, message handling or sport rules changed.

| State | Dark | Light |
| --- | --- | --- |
| Selected team and roster | [Capture](teams-dark.png) | [Capture](teams-light.png) |
| Long jersey and unbroken player name | [Capture](long-labels-dark.png) | [Capture](long-labels-light.png) |
| Empty library | [Capture](empty-dark.png) | |
| Jersey editor | | [Capture](editor-light.png) |
| Archived team | | [Capture](archived-light.png) |

Native checks performed: empty library, team selection through dropdown, selected team highlight, independent list scrolling, long team name, 12-character jerseys, unbroken player name, blank jersey/positions, disabled add action with no player selected, Edit jersey and saved update (24), archive selected team, show archives, restore, reopen roster with eight memberships retained, and full app restart retaining the edited jersey. Light/dark layouts were inspected at minimum size. The native AX tree exposes only the window, so these checks used actual rendered controls.

Local verification passed (exit 0):

```sh
PATH=/Users/jaylupos/.cargo/bin:$PATH \
CARGO_TARGET_DIR=/Users/jaylupos/Desktop/Projects/Web/HustlX-Apps/hustlx-oss/target \
bash scripts/check.sh
```

This includes formatting, 90 Rust tests, Clippy with warnings denied, 9 Python release-safety tests, native debug build and packaging dry-run. The existing dependency warning for `block 0.1.6` future compatibility remains. GitHub Actions remains disabled under the local-only policy. Windows native rendering was not tested.
