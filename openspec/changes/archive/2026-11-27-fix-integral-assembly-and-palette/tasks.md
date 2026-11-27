# Tasks

## 1. Shared integral assembly

- [x] 1.1 Read the STIX MATH vertical assembly for U+222B and use its connector glyph and overlap metrics for extended integral stems. Exercise a target height requiring an extender and verify continuous stem width at both joins.
- [x] 1.2 Assemble both directional integrals with their respective STIX ring glyphs centered on the body. Add a regression check that verifies the direction of each loop and distinguishes the two glyphs from the plain loop with a standalone sign, a side limit, and a stacked limit.
- [x] 1.3 Include body and loop ink in the operator layout and canvas hit bounds. Check that double and triple contour loops and both directional arrowheads fit their own bounds when adjacent to other nodes.

## 2. Palette assets

- [x] 2.1 Correct the six SVG icon variants through the shared integral builder and regenerate the SVGs, PNG atlas, and JSON manifest. Run `python3 scripts/generate_palette_icons.py --check` and inspect the six atlas crops at the displayed `40x26` size for legible direction and clear limit slots.
- [x] 2.2 Verify each affected tile resolves its generated icon by catalog source and inserts the expected operator with the requested limit placement. Exercise the atlas coverage check and the standalone, side-lower, and stacked-lower insertion paths.

## 3. Integration verification

- [x] 3.1 Compare the three contour signs on the desktop canvas at 100% and 200% zoom and in PNG, SVG, and EMF previews. Check direction, clear loop interiors, and consistent placement across the outputs.
- [x] 3.2 Follow the EqnTide precommit workflow and verify `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` pass.
