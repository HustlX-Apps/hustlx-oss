# HustlX branding

The desktop app reuses the existing HustlX identity and ships its assets locally. It makes no font-CDN or logo requests.

## Typography

| Role | Family and weight |
| --- | --- |
| Body, fields, metadata, ordinary controls | Inter Regular, 400 |
| Navigation | Inter SemiBold, 600 |
| Stat buttons and small emphasis labels | Inter Bold, 700 |
| Page headings and library titles | Exo Bold, 700 |
| Game Day scores, clocks, jerseys, team/player names | Exo Black, 900 |

Register all five TTFs in `src/main.rs`; select roles through `src/brand.rs`. Use the actual family names declared by the files. In particular, the native static Black face declares **Exo Black**, rather than Exo with a different weight. The Inter static files all declare Inter, distinguished by their OS/2 weight classes. Do not substitute a system font or add network font loading.

The native fonts come from the existing HustlX Android and iOS distributions. The web `tailwind.config.ts` maps Inter to `sans` and Exo to `heading`. Asset provenance and retained OFL licenses are in [notices](../THIRD_PARTY_NOTICES.md).

## Wordmark and colors

The navigation uses the official white wordmark on dark backgrounds and navy variant on light backgrounds, with a separate OSS label. SVG support preserves the original artwork and scales without raster blur. Both vectors are embedded using `include_bytes!`.

The palette follows the web `app/assets/css/tokens.css`, `app/theme.ts`, and `tailwind.config.ts`: navy `#313a68`, gold `#ffd700`, graphite dark surfaces, and indigo actions in dark mode. Filled dark controls use indigo 600 (`#4f46e5`) with white text; light controls use navy with white text. Hover colors preserve readable labels. Gold text uses the darker secondary 900 (`#7a5500`) on light backgrounds. Semantic scoring/foul accents remain distinct.

`src/brand.rs` owns fonts, colors, native theme palettes, wordmark selection, and neutral button styling. Keep these centralized when expanding the application. Launcher icons, signed bundles, and installer identity remain part of the packaging slice.

## Library layout

Teams uses a fixed 340px sidebar and a flexible roster panel. Creation and roster fields stay visible while each list scrolls independently. Selected teams use a tinted surface and border; secondary row actions use the neutral button style.

Jersey labels may contain up to 12 characters, including letters. Never render them as large text in a narrow fixed-width slot without wrapping: Iced can paint them over adjacent player names. Use a contained badge, reduce the type size for long labels, and wrap words or glyphs. Team and player names also need glyph fallback for unbroken names.

## Visual evidence

[Current native screenshots and checks](evidence/hustlx-branding/README.md). Older screenshots under native-preview and sqlite-library document their historical implementation states.

[Teams layout and long-label verification](evidence/teams-ui/README.md) records the October 21 native presentation update.
