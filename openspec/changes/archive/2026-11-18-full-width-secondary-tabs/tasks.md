# Tasks

## 1. Shared Full-Width Card Helper & Settings Layout

- [x] 1.1 Add a reusable full-width card helper around `ThemeTokens::group_frame()` in `desktop/src/app.rs` that sets content width to `ui.available_width()`, and verify compilation with `cargo check -p eqntide-desktop`.
- [x] 1.2 Update `settings_ui` in `desktop/src/app.rs` to use `egui::ScrollArea::vertical().auto_shrink([false, false])`, remove the `540.0` width cap on the `Canvas zoom`, `Appearance`, and `Startup focus` cards, and add unit-test geometry assertions verifying all Settings cards span the central-panel width at `1040x560` via `cargo test -p eqntide-desktop`.

## 2. About & Batch DOCX Full-Width Layouts

- [x] 2.1 Update `about_ui` in `desktop/src/app.rs` to render its summary, separator, and `about-info` grid inside a full-width card within `ScrollArea::vertical().auto_shrink([false, false])`, and add unit-test geometry assertions verifying the About card spans the central-panel width at `1040x560` via `cargo test -p eqntide-desktop`.
- [x] 2.2 Update `batch_ui` in `desktop/src/app.rs` to wrap the batch controls in a full-width card, display a full-width drop-zone empty state when `self.files.is_empty()`, configure `ScrollArea::vertical().auto_shrink([false, false])` for the file list, and add unit-test geometry assertions verifying both empty and populated Batch DOCX states span the central-panel width via `cargo test -p eqntide-desktop`.

## 3. Regression Verification

- [x] 3.1 Run the full desktop test suite (`cargo test -p eqntide-desktop`) and verify all existing navigation, theme, persistence, and new secondary-tab width tests pass without warnings or clipping at `1040x560` and wider viewports.
