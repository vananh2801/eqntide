# Tasks: Pre-1.0.0 Readability and Maintainability Refactor

## 1. Refactor `eqntide-editor` Crate

- [x] 1.1 Split `crates/eqntide-editor/src/lib.rs` into `node.rs`, `stops.rs`, `navigation.rs`, `editing.rs`, `deletion.rs`, `structures.rs`, and `tests.rs` with top-down declaration order, and verify `cargo test -p eqntide-editor --locked` passes.
- [x] 1.2 Rename `Node::token`, `Node::row`, `Node::opaque`, and `Node::math` to `is_token`, `is_row`, `is_opaque`, and `to_math_node`, update all workspace callers in `desktop/`, standardize internal helper/variable names, and verify `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- [x] 1.3 Add English `//!` module architecture and `///` doc comments (including the `NodeId`/snapshot `transact()` and `canonical_stops()` workflow diagram in `lib.rs`), and verify `cargo fmt --all --check` and `cargo doc -p eqntide-editor --no-deps` succeed without warnings.

## 2. Complete `eqntide` Core MTEF, OMML, and Service Splits

- [x] 2.1 Consolidate `src/mtef/decode.rs` into `src/mtef/decode/{mod,consts,stream,entry,parse,v5,v3,templates,characters,embellishment,tests}.rs` with English `//!` decode-pipeline docs and `///` comments, and verify `cargo test --lib mtef::decode --locked` passes.
- [x] 2.2 Consolidate `src/omml/encode.rs` into `src/omml/encode/{mod,fences,scripts,normalization,nary,accents,font,tests}.rs` with English `//!` encode-pipeline docs, top-down ordering, and descriptive variable names, and verify `cargo test --lib omml::encode --locked` passes.
- [x] 2.3 Clean up `src/mtef/encode/` and `src/mtef/templates.rs` (separate constants from template builders, standardize names, add English `///` docs), and verify `cargo test --lib mtef --locked` passes.
- [x] 2.4 Replace `#[path = "latex_extensions.rs"]` in `src/service.rs` with `src/service/mod.rs` and `src/service/latex_extensions.rs`, document the LaTeX preprocessing and service dispatch flow in `//!` headers, and verify `cargo fmt --all --check && cargo clippy -p eqntide --all-targets --locked -- -D warnings && cargo test -p eqntide --locked` passes.

## 3. Refactor `eqntide` Bridge, Docx, Render, and Catalog Modules

- [x] 3.1 Split `src/bridge/server.rs` into `src/bridge/server/{mod,auth,http,session_routes,convert_routes,addin_assets,tests}.rs` with English `//!` HTTP/session/auth flow docs and `///` comments, and verify `cargo test --lib bridge --locked` passes.
- [x] 3.2 Standardize variable names (`b` -> `layout_box`, `f` -> `font_set`, etc.), top-down ordering, and English `//!` layout-to-rasterize/EMF/SVG pipeline comments across `src/render/mod.rs` and `src/render/layout/`, and verify `cargo test --lib render --locked` passes.
- [x] 3.3 Reorder and document `src/docx/{pipeline,to_omml,upgrade}.rs`, `src/mathml_to_latex.rs`, and `src/catalog.rs` (extracting `catalog.rs` tests if helpful, adding English `//!` pipeline comments and `///` docstrings), and verify `cargo fmt --all --check && cargo clippy -p eqntide --all-targets --locked -- -D warnings && cargo test -p eqntide --locked` passes.

## 4. Refactor `desktop` Crate (`app.rs` and Supporting Modules)

- [x] 4.1 Move the inline `mod tests` block (~5,535 lines) out of `desktop/src/app.rs` into `desktop/src/app/tests/{mod,helpers,session_tests,canvas_input_tests,layout_theme_tests,batch_settings_tests}.rs` and verify `cargo test -p eqntide-desktop --locked` passes.
- [x] 4.2 Split production code in `desktop/src/app.rs` into `desktop/src/app/{mod,events,session,canvas,keyboard,palette_ui,batch_ui,settings_ui}.rs` keeping `App` fields grouped and documented in `app/mod.rs` with an ASCII lifecycle/async flow diagram, and verify `cargo test -p eqntide-desktop --locked` passes.
- [x] 4.3 Standardize `App` method names (`draw_*`, `handle_*`, `process_*`, `sync_*`, `hit_test_*`), replace abbreviated locals (`a` -> `app`, etc.), add English `//!` and `///` comments across `desktop/src/app/` and `desktop/src/{batch,commands,settings,theme,palette,runtime,wgpu_runner}.rs`, and verify `cargo fmt --all --check && cargo clippy -p eqntide-desktop --all-targets --locked -- -D warnings && cargo test -p eqntide-desktop --locked` passes.

## 5. Workspace-Wide Release Audit and Verification

- [x] 5.1 Audit comments and docstrings across `src/`, `crates/eqntide-editor/`, and `desktop/` to confirm all internal comments are in English and every major module documents its operational workflow, verifying `cargo doc --workspace --no-deps` completes with zero broken intra-doc link warnings.
- [x] 5.2 Run the full CI verification suite (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`) and confirm all checks pass.
