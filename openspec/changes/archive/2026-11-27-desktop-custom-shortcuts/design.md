# Design

## Context

Desktop keyboard shortcuts were previously hardcoded in two locations:
- [`desktop/src/app/session.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/session.rs) (`take_save_shortcut`) intercepted `Ctrl/Cmd+S` at the start of `draw_ui()` before widget event processing, and `draw_save_button` embedded `(Ctrl+S)` directly in the button face label without a hover tooltip.
- [`desktop/src/app/keyboard.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/keyboard.rs) (`handle_canvas_keyboard`) matched `Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`, `Ctrl+A`, clipboard events, canvas navigation (`Arrows`, `Home`, `End`, `Tab`, `Enter`, `Backspace`, `Delete`, `Esc`), and 15 template shortcuts.

Three mismatches existed in the prior codebase:
1. Three hardcoded template strings in `keyboard.rs` (`Ctrl+Shift+I`, `Ctrl+Shift+S`, `Ctrl+Shift+P`) differed slightly from the canonical `PALETTE` entries (`\int\limits_{...}`, `\sum\limits_{...}`, `\prod\limits_{...}`).
2. Ten labels in [`src/catalog/mod.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/src/catalog/mod.rs) embedded hardcoded `(Ctrl+...)` suffixes (including `Right vector (Ctrl+Alt+V)`, which was never wired in `keyboard.rs`).
3. `PALETTE` contains ~530 entries across 13 `UI_GROUPS`, each uniquely identified by its canonical `source` string (`by_source.len() == PALETTE.len()` verified by tests in `desktop/src/palette.rs`) and pre-rendered in `PaletteAtlas`.

See `proposal.md` for motivation and `specs/` for exact behavioral requirements.

## Goals / Non-Goals

**Goals:**
- Model shortcuts cleanly in a dedicated `desktop/src/shortcuts.rs` module covering fixed **System & Navigation** entries, canonical **Palette** default bindings, chord parsing/formatting, validation, conflict detection, and lookup for canvas dispatch.
- Persist palette shortcut customizations in `Settings` (`settings.json`) using canonical `PALETTE` `source` keys while preserving explicit clearing of defaults.
- Render the two-column (`30%` / `70%`) **Keyboard Shortcuts** card directly inside `Settings` (`desktop/src/app/shortcuts_ui.rs`) with a unified header row, borderless matching `panel_bg` columns, fast search/category filtering, enlarged palette atlas icons + LaTeX rows, and a fixed-width solid scrollbar with boundary wheel containment.
- Reuse the `Custom matrix` modal presentation and `ModalFocusState` infrastructure in `desktop/src/app/dialogs.rs` for the **Shortcut conflict** modal.
- Keep editor action and palette button faces free of shortcut text or badges, showing shortcuts exclusively in hover tooltips.

**Non-Goals:**
- Rebinding System/Navigation shortcuts (`Ctrl+S`, `Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`, `Ctrl+A`, `Ctrl+C`/`X`/`V`, `Tab`, `Shift+Tab`, `Esc`).
- Binding multiple chords to one palette button or binding single-character/Shift-only shortcuts that would break direct equation typing.
- Adding arbitrary user-defined LaTeX macro snippets outside `PALETTE`.

## Decisions

### 1. Canonical Chord Model & Serialization (`desktop/src/shortcuts.rs`)
- **Decision:** Represent a customizable shortcut as `ShortcutChord { command: bool, alt: bool, shift: bool, key: ShortcutKey }` (serializable via `serde` as clean human-readable strings like `"Ctrl+F"` or `"Ctrl+Shift+R"` via custom `Serialize`/`Deserialize`, validated against a fixed supported `egui::Key` subset: `A..=Z`, `0..=9`, `F1..=F12`, and safe punctuation such as `[` / `]` / `,` / `.` / `;` / `-` / `=` / `/` / `\` / `` ` `` / `'` / `+`).
- **Validation rule:** Require `command || alt || is_function_key` so bare letters, digits, Shift+letters, `/`, `^`, `_`, `\`, `{`, `}`, `Tab`, `Enter`, `Backspace`, `Delete`, and arrow keys can never shadow canvas typing or structural navigation.
- **System table & reservation check:** Define `SYSTEM_SHORTCUTS` in `shortcuts.rs` as 10 individual read-only display rows (`Save equation` -> `Ctrl+S`, `Undo` -> `Ctrl+Z`, `Redo` -> `Ctrl+Y / Ctrl+Shift+Z`, `Select all` -> `Ctrl+A`, `Copy` -> `Ctrl+C`, `Cut` -> `Ctrl+X`, `Paste` -> `Ctrl+V`, `Next placeholder` -> `Tab`, `Previous placeholder` -> `Shift+Tab`, `Close popup / Exit` -> `Esc`), paired with `SYSTEM_RESERVED_CHORDS` listing exact reserved `ShortcutChord`s (`Ctrl+S`, `Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`, `Ctrl+A`, `Ctrl+C`, `Ctrl+X`, `Ctrl+V`). Recording any reserved chord triggers a read-only system conflict.
- **Alternative considered:** Serializing raw `egui::KeyboardShortcut` or `egui::Key` debug names directly with `serde`. Rejected because `egui::Key` enum variants could drift on dependency updates and we want stable, readable strings (`"Ctrl+Shift+R"`) in `settings.json` plus strict validation on load.

### 2. Persistence Using Explicit Overrides Map (`desktop/src/settings.rs`)
- **Decision:** Add `pub(crate) palette_shortcuts: BTreeMap<String, Option<String>>` to `Settings`, storing **overrides relative to defaults**:
  - Key absent from `palette_shortcuts` -> use default shortcut (if any).
  - Key mapped to `Some(chord)` -> bind `chord` to that `PALETTE` `source`.
  - Key mapped to `None` (`null` in JSON) -> explicitly unassigned (keeps cleared default shortcuts cleared across restarts).
  - **Reset to defaults** clears `palette_shortcuts` back to empty (`BTreeMap::new()`).
- **Sanitization on load (`Settings::sanitize`):**
  1. Drop keys whose `source` is not in `palette::canonical_source`.
  2. Drop `Some(chord)` values that fail `ShortcutChord::parse` / `is_valid_palette_chord()` or collide with `SYSTEM_RESERVED_CHORDS`.
  3. Deduplicate colliding `Some(chord)` bindings across the resolved full table so two palette buttons can never end up bound to the same chord (explicit overrides take precedence over defaults; first valid override wins).
  4. Prune redundant `Some(default_chord)` overrides or `None` overrides on entries that have no default, keeping `settings.json` compact.
- **Alternative considered:** Storing the full ~530-item map or only non-null bindings. Storing only non-null bindings cannot distinguish "never configured (use default)" from "user explicitly cleared `Ctrl+F`". An override map with `Option<String>` solves that cleanly while keeping `settings.json` tiny for typical users.

### 3. Runtime Dispatch & `Ctrl+S` Capture Interaction (`app/mod.rs`, `keyboard.rs`)
- **Decision:** Store resolved `shortcuts: ShortcutMap` (including an `O(1)` `HashMap<ShortcutChord, &'static str>` reverse index and `HashMap<&'static str, ShortcutChord>` forward index) on `App`, rebuilt whenever `apply_settings`, rebind, replace, swap, clear, or reset runs.
- In `draw_ui()`, skip `Self::take_save_shortcut(&ctx)` whenever `self.shortcut_capture.is_some() || self.shortcut_conflict.is_some()` so pressing `Ctrl+S` during key capture reaches the Settings recorder and opens the system-conflict modal instead of triggering Save.
- In `handle_canvas_keyboard()`, after `handle_command_key`, if `egui::Event::Key { pressed: true, .. }` matches a chord in `shortcuts`:
  - Match `command`, `alt`, and `shift` strictly.
  - Execute the exact palette insertion flow (`command_input = None`, clear `latex_groups` / `expects_latex_group`, `record_recent_palette(source)`, `insert_template(source)`).
  - Keep `Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`, `Ctrl+A` checked before or disjoint from palette chords (guaranteed by `SYSTEM_RESERVED_CHORDS`).
- **Alternative considered:** Running customizable palette shortcuts globally even when `LaTeX source` is focused. Rejected because `LaTeX source` is a plain multiline `egui::TextEdit` where unexpected template insertions into the AST would conflict with buffered `draft_dirty` text.

### 4. Settings UI Layout, Solid Scrollbar Convention & Conflict Modal (`shortcuts_ui.rs`, `dialogs.rs`)
- **Decision:** Inside `draw_settings_tab()`, add a full-width **Keyboard Shortcuts** card (`desktop/src/app/shortcuts_ui.rs`) structured as:
  - **Single-row header (`30px` height):** Left-aligned `"Keyboard Shortcuts"` strong label; right-aligned controls in left-to-right visual order: **`Reset to defaults`** button (disabled when already at defaults) -> **`Search label, LaTeX, or shortcut…`** `TextEdit` (`240px`) -> **`All categories`** `ComboBox` (`196px`, offering `All categories`, `Assigned only`, and all `UI_GROUPS` excluding `Recent`).
  - **Two-column body (`30%` left / `70%` right, `16px` column gap, zero outer frame margin):** Both columns render inside borderless `panel_bg` frames (`Stroke::NONE`, `CONTROL_CORNER_RADIUS`, `16px` inner padding, matching `360px` height), omitting separate group subheadings:
    - **Left column (30% — System & Navigation):** Non-scrolling borderless `panel_bg` container (`min_height = 360.0`) listing the 10 individual `SYSTEM_SHORTCUTS` entries (`32px` row height, `4px` vertical spacing, no divider lines). Each row left-aligns the truncated action label (with tooltip on hover) and right-aligns a borderless `control_bg` shortcut badge (`CONTROL_CORNER_RADIUS`, `8×2` padding, small monospace text). Because this column has no inner `ScrollArea`, mouse-wheel scrolling over the left 30% area scrolls the outer Settings `ScrollArea` seamlessly.
    - **Right column (70% — Palette Shortcuts):** Optional capture status/error banner above a borderless `panel_bg` container housing a `360px`-tall `ScrollArea::vertical()` (`min_scrolled_height(360.0)`, `max_height(360.0)`, `auto_shrink([false, true])`) with `show_rows()` at `36px` row height. Each row displays:
      - Enlarged `44.0×28.6` `PaletteAtlas` icon (`SHORTCUT_ICON_SIZE`, matching the `40:26` atlas aspect ratio) with hover tooltip showing `row.label`
      - Canonical LaTeX `source` string in primary monospace text (`tokens.text_primary`, omitting `row.label` from the row face while keeping `row.label` searchable via `matches_query` and visible in hover tooltips)
      - Right-aligned shortcut capture button (`Set shortcut` / monospace chord / `Press shortcut…`, `128×30`, borderless at rest with `1px` accent outline while recording) + clear `[×]` button (`28×30`, visible when assigned)
    - **Right-list scroll-boundary isolation:** Immediately after the right `Frame` (and its inner `ScrollArea`) completes inside `draw_palette_shortcuts_list`, if `ui.is_enabled() && ui.rect_contains_pointer(frame_resp.response.rect)`, `input.smooth_scroll_delta.y` is cleared to `0.0`. Because `egui::ScrollArea::end()` evaluates wheel chaining on the outer `ScrollArea` afterward, scrolling up at the top or down at the bottom of the right list never propagates out to the enclosing Settings `ScrollArea`, while wheeling over the left 30% column or elsewhere in Settings continues to scroll the page normally.
  - **Fixed non-floating solid scrollbar convention (prototype for app-wide desktop scrollbar styling):** Inside `draw_palette_shortcuts_list`, override `ui.spacing_mut().scroll` with `egui::style::ScrollStyle::solid()`:
    - `floating = false` (reserves dedicated layout space outside the scroll viewport instead of overlaying content)
    - `bar_width = 8.0` (constant width—does not thin to `floating_width` when unhovered or expand on pointer hover)
    - `bar_inner_margin = 10.0` (clean gap separating the rightmost row buttons `[×]` / shortcut button from the scrollbar track)
    - `bar_outer_margin = 0.0` (respecting the enclosing container's `16px` right inner padding)
    - Combined with `ScrollBarVisibility::AlwaysVisible`, the handle stays fully opaque (`handle_opacity = 1.0` for solid mode in `egui`) and fixed-width regardless of hover state. Future changes can lift this `ScrollStyle::solid()` configuration into `ThemeTokens::apply_style_with_accent` in `desktop/src/theme.rs` for consistent app-wide scrollbars.
- **Conflict Modal (`ShortcutConflictDialog` in `dialogs.rs`):**
  - Triggered when the captured chord matches a system command or another palette `source`.
  - Uses `egui::Modal` + `modal_frame` (`20px` padding) + `MODAL_BUTTON_PADDING` (`14×6`) + `theme::BUTTON_HEIGHT` (`30px`) + `ModalFocusState` (`Tab`, `Shift+Tab`, `ArrowLeft`/`ArrowRight`, `Esc` to cancel, initial focus on primary button).
  - Disable underlying `draw_shell` controls while `self.shortcut_conflict.is_some()` just like `self.matrix_dialog.is_some()`.
  - Actions:
    - System conflict: `"Shortcut reserved"` heading, message naming the locked system action, `[Cancel]` only.
    - Palette conflict: `"Shortcut already in use"` heading, message showing the conflicting item's label + LaTeX, `[Swap]` (only if target already had an old shortcut), `[Replace]`, `[Cancel]`.

### 5. Tooltip-Only Shortcut Hints & Clean Button/Catalog Labels (`src/catalog/mod.rs`, `palette_ui.rs`, `session.rs`)
- **Decision:**
  - In [`desktop/src/app/session.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/session.rs), change `draw_save_button` labels from `"Save & Return to Document (Ctrl+S)"` / `"Save MathML (Ctrl+S)"` to `"Save & Return to Document"` / `"Save MathML"` and attach `.on_hover_text("Save & Return to Document (Ctrl+S)")` / `.on_hover_text("Save MathML (Ctrl+S)")` so the shortcut appears only on hover.
  - Keep `Undo` and `Redo` footer buttons icon-only with `.on_hover_text("Undo (Ctrl+Z)")` / `.on_hover_text("Redo (Ctrl+Y)")`.
  - Keep palette toolbar buttons icon-only (never rendering shortcut text or badges onto the `46×32` button face), strip hardcoded `(Ctrl+...)` suffixes from `src/catalog/mod.rs` labels, and append the active shortcut from `self.shortcuts.binding_for(source)` inside `palette_ui.rs`'s `on_hover_ui` tooltip whenever assigned.
- **Test impact:** Headless UI/layout tests in `desktop/src/app/tests/layout_theme_tests.rs` match clean `"Save MathML"` / `"Save & Return to Document"` labels, and `desktop/src/app/tests/shortcut_tests.rs` verifies the two-column `30%` / `70%` layout, borderless panels and keycap badges, enlarged icon + LaTeX-only palette rows, solid fixed-width scrollbar clearance, right-list top/bottom wheel containment, outer Settings scrollability over the left column, capture/conflict/reset flows, tooltip-only hints, and canvas dispatch.

## Risks / Trade-offs

- **[Inner ScrollArea inside outer Settings ScrollArea]** → Keeping the left 30% column non-scrolling ensures users can always wheel-scroll the outer Settings page over the left column even when the pointer is beside the right 70% list. Inside the right 70% box, `min_scrolled_height(360.0)` / `max_height(360.0)` with `show_rows()` keeps frame times fast across ~530 palette entries, `ScrollStyle::solid()` guarantees fixed-size scrollbar ergonomics without overlapping row buttons, and consuming `smooth_scroll_delta.y` over `frame_resp.response.rect` prevents accidental outer Settings scrolling when hitting top or bottom bounds.
- **[Platform Command key (`Ctrl` vs `Cmd` on macOS)]** → Storing canonical `"Ctrl+..."` in JSON and mapping `"Ctrl"` to `egui::Modifiers::command` (`Ctrl` on Windows/Linux, `Cmd` on macOS) matches existing `take_save_shortcut` and `keyboard.rs` behavior across platforms.
- **[Key capture vs OS/WM shortcuts]** → Window-manager shortcuts (e.g. `Alt+F4`, `Super+...`) are intercepted by the OS before reaching `egui`. Handling standard `Ctrl`/`Alt`/`Shift` + key and `F1`–`F12` covers all in-app needs cleanly.
