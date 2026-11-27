# Tasks

## 1. Palette Subgroup Data Model (`desktop/src/palette.rs` & Catalog)

- [x] 1.1 Define explicit subgroup ordering in `desktop/src/palette.rs` (including `Sums`, `LabeledArrows`, and `Spaces`, splitting 2-item pairs across `Fences`, `Matrices`, and `Bars` so symmetric pairs stay together, curating `Fractions`, `Integrals`, `Matrices`, `Boxes`, `Embellishments`, and `Functions`, and regenerating the 526-entry palette SVGs/atlas) and expose `subgroups_for_ui_group` while keeping `items_for_ui_group` consistent; verify with unit tests in `desktop/src/palette.rs` and `tests/python/test_icons.py`.
- [x] 1.2 Ensure `subgroups_for_ui_group(RECENT_GROUP, recent)` returns all MRU items as a single subgroup so `Recent` preserves continuous 3-row packing; verify with unit tests in `desktop/src/palette.rs`.

## 2. Three-Row Subgroup Ribbon Layout (`desktop/src/app/palette_ribbon.rs`)

- [x] 2.1 Update `draw_palette_grid` and tile placement in `desktop/src/app/palette_ribbon.rs` to lay out subgroups sequentially starting at row 0 of a fresh column, using `PALETTE_GAP` (`4px`) between columns within a subgroup and `theme::SPACE_LG` (`12px`) between distinct subgroups without drawing divider lines, and placing `Custom matrix` at row 0 of a trailing column in `MatricesAndBoxes`.
- [x] 2.2 Update `palette_fills_columns_in_catalog_and_recent_order`, `button_height_tiers_align_footer_tabs_and_compact_controls`, and `palette_scrolls_horizontally_with_either_wheel_axis`, and add tests in `desktop/src/app/tests/palette_tests.rs` verifying `Fractions` `[6, 2, 2]`, 5-item family breaks in `SumsAndBigOps`, 2-item pair top-two-row placement in `MatricesAndBoxes`/`AccentsAndBars`, wider inter-subgroup spacing, trailing `Custom matrix` column placement, continuous `Recent` packing, and horizontal scroll clamping.

## 3. Workspace Verification

- [x] 3.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and `python3 -m unittest discover -s tests/python -p "test_*.py"`, and confirm all checks pass with zero warnings.
