# Design: Refactor Desktop UI Architecture

## Context

See `proposal.md` for motivation. Currently `desktop/src/theme.rs` already centralizes design tokens (`ThemeTokens`, `CanvasPalette`), spacing/radius/height constants, and global `egui::Style` configuration, while `desktop/src/app/` holds all `App` state, views, and tests (`desktop/src/app/tests/*`, ~6,400 lines relying on `use super::super::*;` and calling `App` entrypoints such as `draw_shell`, `draw_dialogs`, `active_canvas_palette`, and `handle_canvas_keyboard`).

However, custom `egui` widgets and per-widget visual overrides grew organically inside `app/` submodules (`palette_ui.rs`, `mod.rs`, `batch_ui.rs`, `settings_ui.rs`, `dialogs.rs`, `shortcuts_ui.rs`), creating cross-feature coupling and duplicated workarounds (`ui.set_opacity(1.0)`, `ui.visuals_mut()` patching, `MODAL_BUTTON_PADDING`, scroll-chaining suppression, and wheel value snapping).

## Goals / Non-Goals

**Goals:**
- Establish a clean 3-layer separation inside `eqntide-desktop`:
  1. `theme.rs` — Tokens, palettes, accent derivation, global `egui::Style`.
  2. `ui/` — Pure, `App`-independent `egui` widgets, icon asset/texture cache, scroll/wheel helpers, and modal focus-trap/frame utilities.
  3. `app/` — `App` state, window shell (`shell.rs`), startup GPU/font setup (`gpu.rs`), background lifecycle (`background.rs`), and feature views (`canvas.rs`, `palette_ui.rs`, `palette_ribbon.rs`, `batch_ui.rs`, `settings_ui.rs`, `shortcuts_ui.rs`, `about_ui.rs`, `dialogs.rs`).
- Keep every non-test `.rs` file within cohesive single-responsibility bounds (~200–800 lines, except `canvas.rs` which is intentionally deferred).
- Preserve all existing `desktop/src/app/tests/*` behavior and test entrypoints with minimal test churn.

**Non-Goals:**
- Splitting `desktop/src/app/canvas.rs` (geometry/hit-testing vs. egui canvas rendering) — explicitly deferred to a future change if needed.
- Changing any visual appearance, color tokens, pixel sizes, keyboard shortcuts, or `openspec/specs/desktop-*` observable behavior.
- Introducing `unsafe` code, `#[path = "..."]` attributes, or public APIs outside `desktop`.

## Decisions

### 1. Standalone `desktop/src/ui/` Module Layer (Decoupled from `App`)

**Choice:** Extract reusable UI controls and `egui` overrides into `desktop/src/ui/` as pure functions/structs taking `(&mut egui::Ui, ThemeTokens, ...)` (or `&mut IconCache` for icons), instead of keeping them as `impl App` helper methods inside `app/`.

**Target layout:**

```text
desktop/src/
├── theme.rs            # ThemeTokens, CanvasPalette, global egui::Style (unchanged role)
├── ui/
│   ├── mod.rs          # Re-exports ui submodules cleanly as pub(crate)
│   ├── icons.rs        # IconCache (HashMap<&'static str, TextureHandle>) + tabler_icon_bytes
│   ├── widgets.rs      # primary_button*, standard_button, compact_button,
│   │                   # with_selectable_style, accent_checkbox, full_width_card,
│   │                   # level_drag_value, section_separator, progress_bar, back_button
│   ├── scroll.rs       # suppress_scroll_chaining, step_snapped_value, ctrl_wheel_steps
│   └── modal.rs        # modal_frame, MODAL_BUTTON_PADDING, ModalFocusState,
│                       # run_modal_dialog / escape + Tab/Arrow focus trap helpers
└── app/
    ├── mod.rs          # App struct, new_with_state, draw_ui, eframe::App
    ├── gpu.rs          # GpuProfile, VM/software adapter detection, font registration
    ├── shell.rs        # draw_shell, draw_sidebar, draw_footer, draw_status_bar, window min size
    ├── background.rs   # background image picker, load/upload, software bg, paint_app_background
    ├── palette_ui.rs   # PaletteAtlas loading, template insertion, recent palette tracking
    ├── palette_ribbon.rs
    ├── settings_ui.rs  # Settings tab UI + apply_settings / flush_settings
    ├── shortcuts_ui.rs
    ├── batch_ui.rs
    ├── about_ui.rs
    ├── dialogs.rs      # Custom matrix, shortcut conflict, finish equation modals (via ui::modal)
    ├── canvas.rs       # Unsplit; calls ui::scroll::* where shared
    ├── keyboard.rs
    ├── events.rs
    └── session.rs
```

**Rationale:**
- Keeping `ui/` independent of `App` enforces a one-way dependency (`app/` -> `ui/` -> `theme.rs`). New widgets cannot accidentally read or mutate `App` session/batch state, and future tabs never need to import `palette_ui.rs` or `batch_ui.rs` for generic buttons.
- **Alternative considered:** Reorganizing helpers inside `desktop/src/app/widgets.rs` as `impl App` methods. Rejected because it keeps generic widgets coupled to `App`'s 60-field struct and makes it harder to enforce clean boundaries over time.

### 2. Icon Asset & Cache Ownership (`ui::icons::IconCache`)

**Choice:** Move `tabler_icon_bytes` (from `app/mod.rs`) and `ui_icon` (from `app/palette_ui.rs`) into `ui::icons::IconCache`, stored as `pub(super) ui_icons: IconCache` on `App` (with a thin `#[cfg(test)]` accessor preserved on `App` if existing tests check `ui_icon_texture_id`).

**Rationale:** Removes 60+ lines of `include_bytes!` asset tables from `app/mod.rs` and eliminates the need for `App` to carry raw `HashMap` lookup logic alongside frame orchestration.

### 3. Centralizing `egui` Workarounds in `ui/`

**Choice:** Consolidate recurring `egui` workarounds into explicit helpers in `ui/`:
- **Crisp disabled custom controls:** `ui::widgets::without_double_dimming(ui, |ui| ...)` (encapsulating `ui.set_opacity(1.0)` inside `add_enabled_ui(false)`) used by `accent_checkbox`, `level_drag_value`, and `draw_opacity_slider_row`.
- **Wheel & scroll-chaining suppression:** `ui::scroll::suppress_scroll_delta_y(ctx)` and `ui::scroll::step_snapped_value(...)` shared by `level_drag_value`, `shortcuts_ui` list container, and `canvas.rs`.
- **Selectable / primary / recording strokes:** `ui::widgets::with_selectable_style(...)`, `primary_button_with(...)`, and a small `with_accent_border(...)` helper for shortcut recording instead of inline `ui.visuals_mut()` blocks in `shortcuts_ui.rs`.
- **Modal focus trap & padding:** `ui::modal::ModalFocusGuard` / helper functions owning `MODAL_INNER_MARGIN`, `MODAL_BUTTON_PADDING`, `modal_frame`, Escape handling, and Tab/Arrow cycling.

**Rationale:** Future `eframe`/`egui` upgrades (e.g., changes to `ScrollStyle`, `Modal`, `visuals.widgets`, or `set_opacity`) only need updating in `theme.rs` and `ui/`.

### 4. Preserving `App` Entrypoints & Test Visibility

**Choice:** Keep `App::draw_shell`, `App::draw_dialogs`, `App::draw_matrix_dialog`, `App::draw_shortcut_conflict_dialog`, `App::theme_tokens`, `App::active_canvas_palette`, and `App::software_background` as `pub(super)` / `pub(crate)` methods on `App`, delegating internally to `crate::ui::*` and the extracted `app/*` submodules. Move constants referenced by `desktop/src/app/tests/*` (`MIN_WINDOW_SIZE`, `PALETTE_HEIGHT`, `CONTENT_PADDING`, etc.) as `pub(super)` re-exports in `app/mod.rs` if moved to `shell.rs`.

**Rationale:** `desktop/src/app/tests/*` contains ~6,400 lines of layout, theme, shortcut, batch, and canvas integration tests using `super::super::*`. Keeping `App`'s module-level API stable avoids thousands of mechanical test edits while still proving the refactored UI behaves identically.

## Risks / Trade-offs

- **[Risk] Subtle `egui::Id` or focus-state regressions when moving modal or widget code** -> **Mitigation:** Keep exact `Id` salts (`"app-status"`, `"app-sidebar"`, `"palette-shortcuts-scroll"`, `"custom-matrix-dialog"`, `"shortcut-conflict-dialog"`, `"finish-equation-dialog"`, etc.) unchanged and run `layout_theme_tests`, `shortcut_tests`, and `session_tests` after every extraction step.
- **[Risk] Circular or awkward borrow conflicts when passing `&mut IconCache` alongside `&mut App` fields inside closures** -> **Mitigation:** Expose `self.ui_icons` as a separate struct field on `App` so Rust field-splitting allows `&mut self.ui_icons` simultaneously with `&self.persisted` / `&mut self.category` / `&mut self.batch_rows`.
- **[Risk] Drift between `CODING_CONVENTIONS.md` and actual files** -> **Mitigation:** Include updating `CODING_CONVENTIONS.md` section 2 as a tracked task in the final group, verified alongside `cargo doc --workspace --no-deps`.
