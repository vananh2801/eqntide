# Tasks

## 1. Canvas Overlay Tokens & Transparent Placeholder Styling

- [x] 1.1 Add `CANVAS_OVERLAY_CORNER_RADIUS` (`2px`) and set `CanvasPalette::placeholder_bg` to `Color32::TRANSPARENT` for all light and dark presets in `desktop/src/theme.rs` while keeping `placeholder_stroke` colors unchanged; update and run `cargo test -p eqntide-desktop theme::tests` to verify `placeholder_bg == Color32::TRANSPARENT`.
- [x] 1.2 Update `paint_placeholder` in `desktop/src/app/canvas.rs` to use `CANVAS_OVERLAY_CORNER_RADIUS` (`2px`), paint the `pal.bg` mask only behind the synthetic glyph, and skip opaque interior fill when `pal.placeholder_bg.a() == 0`; add a canvas test in `desktop/src/app/tests/canvas_input_tests.rs` verifying `paint_placeholder` emits dashed stroke shapes with `2px` corner rounding and no tinted interior rect, and run `cargo test -p eqntide-desktop placeholder`.

## 2. Contiguous Span-Based Equation Canvas Selection & Boundary Precision

- [x] 2.1 Replace per-node selection drawing in `desktop/src/app/canvas.rs` with a span-based `selection_rect` helper driven by `Editor::selection_span()` that merges sibling child bounds (`[start..end]`) with baseline line-height bounds (`baseline - 0.72 * em .. baseline + 0.22 * em`) and slices intra-token selections via `carets[start..end]`, painting one `CANVAS_OVERLAY_CORNER_RADIUS` (`2px`) rectangle; snap boundary `caret_rect` X to the moving selection edge (`selection_caret_x`) and split row gap hit-testing at the inter-child midpoint so nudging across spacing gaps never shifts the caret.
- [x] 2.2 Refine `Editor::selection_span()` in `crates/eqntide-editor/src/editing.rs` with `is_at_leading_edge` and `is_at_trailing_edge` boundary resolution so cross-node drag and `Shift+Arrow` selections exclude untouched leading (`offset == 0`) or trailing (`offset == len`) non-empty tokens while preserving empty placeholder and structural slot selection; add regression tests in `crates/eqntide-editor/src/tests.rs`.
- [x] 2.3 Add tests in `desktop/src/app/tests/canvas_input_tests.rs` verifying that selecting `\frac{1}{2}x^2 + 2x - 1 = 0` produces a single `Selection` rect, selecting `-` alone stays vertically aligned with the math baseline rather than hanging downward, selecting a substring inside a token highlights only `[carets[start]..carets[end]]`, dragging into the left half of a token excludes it until past midpoint, and dragging across inter-element spacing keeps the caret bar locked to the active selection edge; run `cargo test -p eqntide-desktop canvas_input_tests`.

## 3. Vertically Centered Status Bar & Vector Readiness Dot

- [x] 3.1 Update `App::draw_status_bar` in `desktop/src/app/mod.rs` to use `ui.horizontal_centered`, allocate a fixed hoverable rect for the bridge indicator, and paint a centered vector circle (`circle_filled` when `bridge_ready`, `circle_stroke` otherwise) alongside vertically centered compact buttons and status labels.
- [x] 3.2 Update `status_bar_is_quiet_when_idle_or_busy_and_clears_stale_messages_on_edit` in `desktop/src/app/tests/layout_theme_tests.rs` (plus vertical centering assertions comparing the vector dot center and status text center inside the `32px` status bar) and run `cargo test -p eqntide-desktop status_bar`.

## 4. Full Workspace Verification

- [x] 4.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` to verify all workspace crates pass cleanly without regressions.
