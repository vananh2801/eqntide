# Tasks

## 1. Palette layout

- [x] 1.1 Render fixed-height columns of up to three tiles in desktop display order, including Recent and the custom matrix button. Verify the first four tile positions across window sizes and check the custom matrix button's position after the last catalog tile.
- [x] 1.2 Remove the palette scrollbar and its height allocation. Update the compact layout checks to assert that the canvas starts at the same height for fitting and overflowing categories, with all three palette rows accessible at the tested DPI scales.

## 2. Palette navigation

- [x] 2.1 Use the full ribbon width for tiles and clamp the offset to the current scrollable range. Verify full-width overflow, access to the last column, and a reset offset after resizing to fit.
- [x] 2.2 Add primary-button horizontal dragging from the viewport and from tiles. Verify that crossing the drag threshold suppresses tile activation on release, including over the original tile and over the custom matrix button, while ordinary clicks still insert templates or open the dialog.
- [x] 2.3 Preserve vertical wheel conversion and horizontal wheel input in the palette. Update the existing wheel tests to check scroll movement without a palette scrollbar and a stationary offset for fitting categories. Verify that drag and wheel input share the same offset and that disabled palette states suppress navigation.

## 3. Workspace verification

- [x] 3.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`. Verify that the existing canvas, LaTeX, and secondary-view scrollbar tests still pass.
