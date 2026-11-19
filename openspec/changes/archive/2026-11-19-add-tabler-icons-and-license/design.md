# Design

## Context

`assets/icons/tabler-icons/` holds 32 Tabler outline SVGs (`24x24` viewBox, `stroke="currentColor"`, `stroke-width="2"`) plus `LICENSE.txt` (MIT License, Copyright (c) 2020-2026 Paweł Kuna). Currently:
- `desktop/Cargo.toml` uses `eframe` and `image` (PNG only) with no SVG rasterizer, and `desktop/src/app.rs` builds a white-on-transparent `PaletteAtlas` (`build_mask_atlas`) tinted at draw time via `egui::Image::tint(...)`.
- `word-template/build_dotm.py` embeds a single `logo.png` (`eqntideLogo`) for all five `customUI14.xml` Ribbon buttons.
- `wps-addin/main.js` returns `"assets/icon-32.png"` for every control in `GetImage(control)`, and `wps-addin/taskpane.html` only has a hand-written refresh `<svg>` on `#btn-start-upgrade`.
- `.github/workflows/ci.yml` runs `cargo test` before packaging, and `.github/workflows/build-native.yml` runs `word-template/build_dotm.py` -> `cargo build` -> `scripts/package_native.py` on both Linux and Windows runners.

See `proposal.md` for scope and `specs/` for behavioral requirements.

## Goals / Non-Goals

**Goals:**
- Keep `desktop/Cargo.toml` free of runtime or build-time SVG dependencies so `cargo test`, `cargo build`, and `cargo-deny` remain lightweight and fast.
- Provide a single deterministic generator (`scripts/render_icons.py`) that converts the selected Tabler SVGs into committed PNG assets usable directly by Rust (`include_bytes!`), Word (`build_dotm.py`), and WPS (`wps-addin/assets/`).
- Preserve theme-aware icon tinting in `eqntide-desktop` (Light/Dark, selected/hovered/disabled) and crisp high-contrast `#1D78B8` Ribbon icons in Word and WPS.
- Document Tabler Icons attribution in `NOTICE` and `README.md`.

**Non-Goals:**
- Runtime SVG parsing inside `eqntide-desktop`.
- Converting the top 3-row equation symbol/template palette (`PALETTE` in `eqntide::catalog`) away from STIX Math rendering.
- Modifying `word-template/EqnTide.bas` (VBA has no custom UI forms; Word Ribbon icons live entirely in `customUI14.xml` and `.rels`).

## Decisions

### 1. Pre-rasterized committed PNG pipeline (`scripts/render_icons.py`, Approach 1A)
- **Choice**: Render the required PNG variants via `scripts/render_icons.py` and commit the output PNGs into the repo (`assets/icons/rendered/` for Desktop and `wps-addin/assets/icons/` or `assets/icons/ribbon/` for Word/WPS). `scripts/package_native.py` and `word-template/build_dotm.py` consume those PNGs directly, with an optional `--check` mode in `scripts/render_icons.py` (callable from tests or packaging) to verify PNGs stay in sync when SVGs change.
- **Why over runtime `resvg` or packaging-time-only conversion**: `cargo test` and `cargo build` run before `package_native.py` in CI, and Windows CI runners lack `libcairo` by default. Committing deterministic PNGs guarantees zero extra system/Rust dependencies during `cargo test`, `cargo build`, `build_dotm.py`, and dev sideloading (`install-addin.*`).
- **Renderer backend for the offline generator**: Use `resvg` CLI or Python `resvg_py` / `cairosvg` inside `scripts/render_icons.py` when regenerating icons locally (documented in the script docstring), while CI and normal builds read the committed PNGs without needing an SVG renderer installed.

### 2. Asset formats per target
- **Desktop (`assets/icons/rendered/<name>.png`)**: Render each used Tabler SVG at `48x48` (`2x` of `24x24`) in **pure white (`#FFFFFF`) on transparent background**. `desktop` loads these bytes via `include_bytes!`, packs or uploads them as `egui` textures with `TextureOptions::LINEAR`, displays them at logical UI sizes (`22x22` for all sidebar category icons including dual-icon pairs with `0px` explicit gap, `16x16` for sidebar tabs/Undo/Redo/Back/Batch/About, `32x32` for Batch empty state), and applies `.tint(color)` using `ThemeTokens`.
- **Word & WPS Ribbon (`32x32` `#1D78B8` PNGs)**: Render the 5 Ribbon icons (`pencil-plus`, `edit`, `tex`, `arrow-big-up-lines`, `file-type-docx`) at `32x32` with stroke color `#1D78B8` on transparent background. Store in `wps-addin/assets/icons/<name>-32.png` (and reuse those exact 5 PNGs inside `word-template/build_dotm.py`).
- **WPS Taskpane (`wps-addin/taskpane.html`)**: Embed the exact Tabler SVG markup (`arrow-big-up-lines` on `#btn-start-upgrade`, `file-type-docx` on `#btn-disk-upgrade`) at `16x16` with `stroke="currentColor"` so CSS button text color controls contrast automatically.

### 3. Desktop sidebar & UI integration
- Replace `palette::UI_GROUPS` `icon_source` math rendering in `sidebar()` with a lookup to the 12 rendered Tabler icon textures (stored in a lightweight `HashMap<&'static str, egui::TextureHandle>` or single icon atlas built once on `App` startup/first frame).
- Keep `palette::UI_GROUPS` catalog mappings, `RECENT_GROUP`, `SIDEBAR_BUTTON_HEIGHT` (`42px`), `SIDEBAR_TAB_HEIGHT` (`34px`), and button height tiers (`30px` standard, `24px` compact) unchanged; widen `app-sidebar` from `158px` to `174px` (preserving `12px` horizontal padding) and `MIN_WINDOW_SIZE` from `1040x560` to `1056x560` so each sidebar category tile becomes `72x42` while keeping the `858px` central editor width for all 49 `Symbols` palette buttons.
- Update `App::ui_icon(name)` helper so `sidebar()` tabs (`Batch DOCX`, `Settings`, `About`), `footer()` (`Undo`, `Redo`), secondary-view `Back` buttons, `batch_ui()` controls/empty state, and `about_ui()` metadata rows (`Author`, `License`, `GitHub`) share one clean helper without duplicating `include_bytes!` or texture loading.

### 4. Word `.dotm` multi-image `customUI14.xml`
- In `word-template/build_dotm.py`, replace the single `eqntideLogo` relationship with 5 image relationships (`iconInsertEq`, `iconEditEq`, `iconToggleLatex`, `iconUpgradeMt`, `iconBatchConv`) pointing to `images/<name>.png` inside `customUI/images/`, and bind each `<button image="..."/>` accordingly. Regenerate `word-template/EqnTide.dotm`.

### 5. Attribution & distribution packaging
- Add a numbered section in `NOTICE` for **Tabler Icons** (`https://github.com/tabler/tabler-icons`, `Copyright (c) 2020-2026 Paweł Kuna`, `MIT License`, `assets/icons/tabler-icons/LICENSE.txt`) and summarize it in `README.md` under **References & Credits** and **License**.
- Keep `assets/icons/tabler-icons/LICENSE.txt` in `assets/icons/` so `scripts/package_native.py` (`shutil.copytree(ROOT / "assets/icons", stage / "icons")`) continues shipping the license alongside `NOTICE` and `LICENSE`. (Note: since `desktop` embeds `assets/icons/rendered/*.png` via `include_bytes!`, verify whether `stage/icons/tabler-icons/` should keep all SVGs + `LICENSE.txt` or only `LICENSE.txt`; keeping `assets/icons/tabler-icons/` intact in `stage/icons` is simple and preserves source attribution, or `package_native.py` can exclude raw `.svg` if package size matters.)

## Risks / Trade-offs

- **Committed PNGs drifting from SVGs** → Include a manifest/hash or optional check in `scripts/render_icons.py --check` (runnable when an SVG renderer is present) and clearly document regeneration in the script header.
- **Existing desktop tests asserting old glyph behavior** → Tests in `desktop/src/palette.rs` and `desktop/src/app.rs` (`ui_groups_cover_every_catalog_group_and_render_visible_icons`, `sidebar_icons_regenerate_on_dpi_change_and_tint_across_themes`, `sidebar_and_palette_icons_render_at_uniform_base_em_scale`, `button_rect_around("Batch DOCX")` etc.) expect specific icon rects/tints; update those tests intentionally to verify Tabler icon textures, tints, and button hitboxes.
- **WPS Ribbon `GetImage` path resolution** → Keep `wps-addin/assets/icons/*.png` relative to the add-in root (`"assets/icons/<name>-32.png"`), matching the existing `"assets/icon-32.png"` convention, and cover `GetImage` mapping in `tests/wps` unit tests.
