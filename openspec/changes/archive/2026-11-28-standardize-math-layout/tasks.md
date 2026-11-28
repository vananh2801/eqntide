# Tasks

## 1. Extend OpenType MATH Constants (`src/render/font.rs`)

- [x] 1.1 Capture baseline visual inspection images by running `cargo test --test export_verification -- --nocapture` and checking `target/test_exports/visual/INDEX.md` before modifying layout code.
- [x] 1.2 Extend `MathConstants` and `fonts()` in `src/render/font.rs` to extract `subscript_top_max`, `subscript_baseline_drop_min`, `superscript_bottom_min`, `superscript_baseline_drop_max`, `superscript_bottom_max_with_subscript`, `space_after_script`, `display_operator_min_height`, `upper_limit_gap_min`, `upper_limit_baseline_rise_min`, `lower_limit_gap_min`, `lower_limit_baseline_drop_min`, `accent_base_height`, `overbar_vertical_gap`, `overbar_rule_thickness`, `overbar_extra_ascender`, `underbar_vertical_gap`, `underbar_rule_thickness`, `underbar_extra_descender`, `skewed_fraction_horizontal_gap`, and `skewed_fraction_vertical_gap` from `STIXTwoMath-Regular.ttf` (with matching `Default` values), and verify via `test_math_constants_and_glyph_info` in `src/render/font.rs`.

## 2. Unify `axis_height` Across Layout Modules

- [x] 2.1 Remove `pub(crate) const AXIS` from `src/render/layout/mod.rs` and replace all `AXIS` references in `bigop.rs`, `delimiter.rs`, `enclosure.rs`, `fence.rs`, `integral.rs`, `matrix.rs`, `row.rs`, and `script.rs` with `self.font_set.constants().axis_height` (or `fonts().constants().axis_height` in unit tests), verifying with `cargo test --lib render::layout`.

## 3. Style-Aware Row Spacing (`src/render/layout/row.rs`)

- [x] 3.1 Update `Layout::spacing` to accept `ctx: MathContext` and suppress `MED`, `THICK`, and non-`Op` `THIN` spacing when `ctx.is_script()` (preserving `Display` and `Text` spacing unchanged), updating callers in `Layout::row`.
- [x] 3.2 Add unit tests in `src/render/layout/row.rs` verifying `Display`/`Text` spacing parity alongside compact `Script`/`ScriptScript` spacing for `x^{a+b}`, `x_{i=1}`, and `A_{i,j}`, and run `cargo test --lib render::layout::row`.

## 4. Visual Inspection & Workspace Verification

- [x] 4.1 Re-run `cargo test --test export_verification -- --nocapture` and inspect `target/test_exports/visual/14_roundtrip_p1.png`, `14_roundtrip_p2.png`, `15_layout_edge_cases.png`, and affected sheets using `view_file` against the 5 criteria in `tests/TESTING.md` to confirm script spacing fits `STIXTwoText` + `STIXTwoMath`.
- [x] 4.2 Run full pre-commit checks (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) and confirm zero warnings or failures.
