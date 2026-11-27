# Proposal

## Why

Desktop equation editing currently relies on ~15 hardcoded template shortcuts in [`desktop/src/app/keyboard.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/keyboard.rs) with no in-app reference or way to rebind, clear, or assign shortcuts to the remaining palette symbols and templates. Adding a two-column keyboard shortcut panel directly in the Settings tab—with fixed system/navigation shortcuts on the left and customizable palette shortcuts on the right plus a conflict-resolution modal—gives users discoverability and flexible one-keystroke palette insertion without risking accidental overrides of core editing commands.

## What Changes

- Add a **Keyboard Shortcuts** full-width card directly in the **Settings** tab with a single-row header (**Keyboard Shortcuts** on the left; **Reset to defaults**, search box, and category filter dropdown right-aligned on the right) and a borderless two-column (`30%` / `70%`) body:
  - **Left column (30% — System & Navigation)**: Non-scrolling borderless `panel_bg` container displaying 10 individual read-only rows (`Save equation`, `Undo`, `Redo`, `Select all`, `Copy`, `Cut`, `Paste`, `Next placeholder`, `Previous placeholder`, `Close popup / Exit`) with left-aligned labels and right-aligned borderless `control_bg` shortcut badges, allowing outer Settings scrolling when the pointer wheels over this column.
  - **Right column (70% — Palette Shortcuts)**: Filterable/searchable borderless `panel_bg` scrollable container (`360px` bounded viewport with a fixed-width, always-visible solid non-floating scrollbar and dedicated right inner margin so the scrollbar never overlaps action buttons) displaying each palette entry's enlarged `44.0×28.6` atlas icon and canonical LaTeX snippet (with human-readable labels in hover tooltips and search matching) plus inline shortcut capture and clear (`×`) controls.
- Standardize the default palette shortcut set onto canonical `PALETTE` entries (`source` keys) and route Equation Canvas shortcut dispatch through the active shortcut map.
- Keep action and palette button faces clean without embedded shortcut text or badges (removing `(Ctrl+S)` from the Save button label in favor of `"Save MathML"` / `"Save & Return to Document"`), and display shortcuts exclusively inside hover tooltips (including dynamic shortcuts for palette buttons and `Ctrl+S` on the Save button tooltip).
- Add a centered **Shortcut conflict** modal dialog (matching the `Custom matrix` modal padding, button sizing, and keyboard focus navigation) when a newly recorded shortcut collides with an existing binding:
  - Collision with a **System & Navigation** shortcut displays a locked-conflict notice with **Cancel** only.
  - Collision with another **Palette** shortcut offers **Cancel**, **Replace** (assigns to target and clears the previous owner), and **Swap** (exchanges bindings when the target also had an existing shortcut).
- Persist customized palette shortcuts (including explicitly cleared defaults) in `settings.json` with safe sanitization on load.

## Capabilities

### New Capabilities
- `desktop-custom-shortcuts`: Two-column (`30%` / `70%`) keyboard shortcut configuration UI in Settings (including solid non-floating fixed-width scrollbar layout), shortcut recording and validation rules, conflict-resolution modal workflow, tooltip-only shortcut hints (with dynamic palette tooltips), and Equation Canvas shortcut dispatch.

### Modified Capabilities
- `desktop-settings-persistence`: Persist, sanitize, and restore customized palette shortcut bindings (including cleared default bindings) in `settings.json`.
- `desktop-modal-dialogs`: Extend centered `egui::Modal` layout and trapped keyboard focus navigation rules to the **Shortcut conflict** confirmation dialog.

## Impact

- **Desktop crates/modules**: `desktop/src/settings.rs`, `desktop/src/app/mod.rs`, `desktop/src/app/settings_ui.rs`, `desktop/src/app/shortcuts_ui.rs`, `desktop/src/app/keyboard.rs`, `desktop/src/app/palette_ui.rs`, `desktop/src/app/dialogs.rs`, `desktop/src/app/session.rs`, and `desktop/src/shortcuts.rs`.
- **Core catalog**: `src/catalog/mod.rs` (decoupling hardcoded `(Ctrl+...)` suffixes from palette tooltip labels while keeping `ENTRIES`/`PALETTE` invariants intact).
- **Tests**: Unit and headless UI/layout tests under `desktop/src/settings.rs`, `desktop/src/shortcuts.rs`, and `desktop/src/app/tests/` (including `shortcut_tests.rs` and `layout_theme_tests.rs`).
