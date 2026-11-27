# Tasks

## 1. Shared theme and scrollable lists

- [x] 1.1 Install the shared solid scrollbar style for Light and Dark and remove the renderer-specific floating scrollbar overrides. Compare idle, hover, and drag in Settings and About to verify constant thickness and legible handles in both themes.
- [x] 1.2 Use overflow-based visibility for Palette Shortcuts and both Batch DOCX list modes. Add or update regression coverage for a short filtered shortcuts list and a Batch folder tree that fits after collapsing rows. Verify that a long list reserves a gutter, the shortcut row controls stay clear of it, and the existing nested wheel tests pass.

## 2. Editor scroll areas

- [x] 2.1 Adopt the shared geometry in Equation Canvas and LaTeX source and preserve canvas-specific scrollbar colors. Verify that Canvas shows the correct bars for no overflow, horizontal only, vertical only, and both axes. Verify that LaTeX stays bounded, can scroll to its last line, and releases the gutter when its content fits again.
- [x] 2.2 Allocate a horizontal gutter below palette tiles only when their grid overflows. Verify all three rows stay visible at `1056x560` and during DPI changes, and that the existing horizontal wheel mapping still works.

## 3. Integration checks

- [x] 3.1 Smoke-test desktop scrollbar visibility and placement on GPU and software renderers, including resize and theme changes. Run the repository's required formatting, Clippy, workspace tests, and rustdoc checks.
