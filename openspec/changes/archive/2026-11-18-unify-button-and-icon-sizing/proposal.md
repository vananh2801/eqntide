# Proposal

## Why

Sidebar category tiles and top palette buttons currently render mathematical symbols using unconstrained scale-to-fit sizing (`render_alpha_fit`), which blows up low-height glyphs (such as `★`, `αΩ`, `∀ ∈`, `± ≤`, and `□^□`) while shrinking multi-tier expressions (`\lim_{□→□}`, `\int □`, `\sum`, matrices) down to tiny proportions. In addition, text buttons across the footer, sidebar tabs, and compact controls mix `15–17px` `small_button`, `25–29px` default buttons, `30px` footer controls, and `38px` sidebar tabs with `13px` text, making controls look visually uneven despite sharing margins.

## What Changes

- Render sidebar category math icons and palette symbol/template icons at a **consistent typographic em size** (`target_em_px`) with downscale-only clamping when an expression exceeds the tile's inner bounds, rather than upscaling short symbols to fill the box.
- Enlarge all uniform palette tiles consistently (`46 x 32 px` with `4px` grid gap and `40 x 26 px` icon budget) and expand sidebar category icon bounds (`54 x 32 px` inside the `64 x 42 px` tile) so wider/taller templates (such as `max_□`, `min_□`, and `\lim_{□→□}`) stay legible at or near the unified base font size while preserving all 49 `Symbols` items inside the `1040x560` viewport without clipping and keeping the palette at three rows.
- Standardize text typography (`text_styles`) and button heights across UI controls: align `Save`, `Undo`/`Redo`, and `DragValue` at `30px` height in the footer, harmonize sidebar navigation tabs (`34px`) and standard action buttons (`30px`), and replace zero-padded `small_button` controls (`+ row`, `− row`, `+ col`, `− col`, `Retry`, `Next session`) with compact `24px` buttons that preserve consistent vertical padding and font scale.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `desktop-ui-theme`: Add consistent button typography and height tiers across standard and compact controls, and require uniform typographic scale for math-rendered button icons without upscaling distortion.
- `desktop-palette-navigation`: Require uniform palette tile sizing and consistent icon typographic scale across sidebar category icons and palette items while maintaining the 3-row and `1040x560` visibility guarantees.

## Impact

- `src/render/mod.rs` & `src/lib.rs`: Add `render_alpha_at(math, target_em_px, max_w_px, max_h_px)` sharing a common helper with `render_alpha_fit` to cap icon scale at `target_em_px / em`.
- `desktop/src/theme.rs`: Configure explicit `style.text_styles` (`Small`, `Body`, `Button`, `Heading`, `Monospace`) and standard/compact button sizing constants (`BUTTON_HEIGHT`, `COMPACT_BUTTON_HEIGHT`).
- `desktop/src/app.rs`: Update `PALETTE_BUTTON_SIZE` (`46x32`), `PALETTE_GAP` (`4.0`), `build_sidebar_atlas` (`target_em = 18.0 * ppp`, `54x32`), `build_palette_atlas` (`target_em = 17.0 * ppp`, `40x26`), `save_button`, `sidebar` bottom tabs, and compact matrix/status buttons; add unit tests verifying uniform glyph em-scale and consistent button heights.
- `desktop/src/palette.rs`: Update palette icon render smoke test to exercise `render_alpha_at`.
