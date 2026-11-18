# Tasks

## 1. Capped Math Icon Scale & Typography Tokens

- [x] 1.1 Add `render_alpha_at(math, target_em_px, max_w_px, max_h_px)` in `src/render/mod.rs` (sharing helper logic with `render_alpha_fit`) and re-export it in `src/lib.rs`, ensuring icons cap at `target_em_px` without upscaling short symbols and only downscale when exceeding bounds; verify with unit tests comparing single-line symbols (`★`, `\alpha`) against multi-tier templates (`\frac{#?}{#?}`).
- [x] 1.2 Configure explicit UI `text_styles` (`Small`, `Body`, `Button`, `Heading`, `Monospace`), `drag_value_text_style`, and button height constants (`BUTTON_HEIGHT = 30.0`, `SIDEBAR_TAB_HEIGHT = 34.0`, `COMPACT_BUTTON_HEIGHT = 24.0`) in `desktop/src/theme.rs`, and verify with unit tests confirming `TextStyle::Button` and `TextStyle::Body` share consistent `13.0px` sizing.

## 2. Desktop Tile, Atlas & Button Consistency

- [x] 2.1 Update `PALETTE_BUTTON_SIZE` (`46x32`), `PALETTE_GAP` (`4.0`), `build_sidebar_atlas` (`render_alpha_at` at `18.0 * ppp` in `54x32`), `build_palette_atlas` (`render_alpha_at` at `17.0 * ppp` in `40x26`), and `desktop/src/palette.rs` icon tests; verify with unit tests confirming uniform icon scale across single-line sidebar/palette glyphs while all 49 `Symbols` and 3 palette rows fit at `1040x560`.
- [x] 2.2 Standardize text button heights in `desktop/src/app.rs` (`primary_button` / `Save` / `Undo` / `Redo` / `DragValue` at `30.0px`, `sidebar` bottom tabs at `34.0px`, optical `15.0px` sizing for `↶`/`↷`, and `compact_button` at `24.0px` replacing `small_button` on matrix and status controls) and verify with headless UI tests confirming aligned button heights in the footer and non-scrolling sidebar at `1040x560`.

## 3. Workspace Verification

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p eqntide`, and `cargo test -p eqntide-desktop` to confirm clean formatting, zero lint warnings, and full test passes across core and desktop crates.
