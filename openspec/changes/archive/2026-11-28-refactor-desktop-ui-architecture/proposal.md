# Proposal: Refactor Desktop UI Architecture

## Why

Rapid iteration on custom `egui` chrome, theming, adaptive background images, modals, and custom controls in `eqntide-desktop` has scattered reusable UI primitives and style overrides across feature modules (`desktop/src/app/palette_ui.rs` hosts shared buttons, checkboxes, cards, and icons; `desktop/src/app/mod.rs` has grown to 1,565 lines mixing state, GPU detection, shell layout, and the level-fill `DragValue` widget; `desktop/src/app/batch_ui.rs` hosts the shared Back button; `desktop/src/app/settings_ui.rs` mixes tab UI with theme resolution and background rendering). Extracting a standalone UI kit and separating window shell, GPU setup, and background management makes the desktop crate clean, modular, and safe to evolve without altering any user-facing behavior.

## What Changes

- Introduce a standalone `desktop/src/ui/` module layer (`ui/mod.rs`, `ui/widgets.rs`, `ui/icons.rs`, `ui/scroll.rs`, `ui/modal.rs`) decoupled from `App`, consolidating all custom `egui` widget wrappers, visual overrides, Tabler icon caching, scroll/wheel helpers, and modal focus-trap infrastructure into one place.
- Slim `desktop/src/app/mod.rs` by extracting window chrome (`shell.rs`: `draw_shell`, `draw_sidebar`, `draw_footer`, `draw_status_bar`) and startup font/GPU tier setup (`gpu.rs`: `GpuProfile`, adapter/VM detection, font registration).
- Extract background image lifecycle, texture upload, software backdrop bridging, and viewport painting from `desktop/src/app/settings_ui.rs` into `desktop/src/app/background.rs`, leaving `settings_ui.rs` focused on the Settings tab controls.
- Return `desktop/src/app/palette_ui.rs` to its intended scope (palette atlas loading, template insertion, recent palette tracking) and move `draw_back_to_editor_button` out of `desktop/src/app/batch_ui.rs` into `ui::widgets`.
- Keep `desktop/src/app/canvas.rs` intact in this change (deferred for a separate split if needed), only reusing shared `ui::scroll` step/wheel helpers where appropriate.
- Update `CODING_CONVENTIONS.md` to document the new `desktop/src/ui/` and `desktop/src/app/` module layout, while preserving all 7-layer ordering, naming, `#![forbid(unsafe_code)]`, and zero-warning verification rules.

## Capabilities

### New Capabilities

<!-- Purely internal structural refactor; no new user-facing capabilities. -->

### Modified Capabilities

<!-- No spec-level behavior changes (`skip_specs: true` in `.openspec.yaml`). -->

## Impact

- **Crates affected**: `eqntide-desktop` (`desktop/src/`) and repository documentation (`CODING_CONVENTIONS.md`).
- **Behavior & Specs**: Zero changes to observable layout, theme colors, keyboard shortcuts, modal interactions, Office bridge behavior, or existing `openspec/specs/desktop-*` contracts.
- **Verification**: Existing `desktop/src/app/tests/*` suites remain intact and all four workspace checks (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) must pass.
