# Tasks

## 1. HTTP Bridge Validation, Path Containment, and Resource Bounds

- [x] 1.1 Implement per-user bridge token generation/persistence (`0600` on Unix, user local data dir) and enforce `X-EqnTide-Token` validation across protected routes in `src/bridge/server.rs`, adding unit tests verifying `401 Unauthorized` without a valid token and `200 OK` with the token.
- [x] 1.2 Tighten `Host`, `Origin`, and CORS handling in `src/bridge/server.rs` (reject missing `Host`, remove `Origin: null` and empty `Origin`, enforce `https://` for remote Office/WPS domains, avoid `*` fallback), and update unit tests in `src/bridge/server.rs` to verify HTTP/1.0 no-Host and `Origin: null` rejections.
- [x] 1.3 Fix `/wps-addin/` path traversal in `serve_addin_file` (`src/bridge/server.rs`) by rejecting non-`Normal` path components, `%` traversal, and paths canonicalizing outside `wps-addin`, with unit tests covering Windows drive-letter (`C:/...`), UNC, and `..` inputs.
- [x] 1.4 Remove `/open/folder` route and `/tmp/last_open_req.xml` debug write in `src/bridge/server.rs`, enforce `.docx` extension validation on `output_path` in `convert_document` (`src/service.rs`), and add unit tests verifying `/open/folder` returns 404 and non-`.docx` `output_path` returns 400.
- [x] 1.5 Validate `session_id` existence and UUID format in `handle_session_update` before rendering/writing `.emf` files, enforce request body size limits (`413`), bounded worker concurrency, and clamped `wait` timeouts in `src/bridge/server.rs`, and wire `SessionManager::cleanup_stale` (`src/bridge/session.rs`) with stale `.emf` cleanup; verify via unit tests in `src/bridge/server.rs`.

## 2. Desktop Single-Instance IPC and Client Token Integration

- [x] 2.1 Remove the shared `std::env::temp_dir()` fallback in `desktop/src/instance.rs` (use restrictive `0700` user data dir permissions on Unix), authenticate IPC `Vec<String>` payloads with a per-user `ipc.secret`, and add unit tests verifying unauthenticated IPC payloads are rejected.
- [x] 2.2 Update `desktop/src/runtime.rs` to read/provide the per-user bridge token in `X-EqnTide-Token` on all internal requests and verify `cargo test -p eqntide-desktop` passes.

## 3. Word VBA Template and WPS JS Add-in Synchronization

- [x] 3.1 Update `word-template/EqnTide.bas` to read the per-user bridge token file and send `X-EqnTide-Token` on all bridge HTTP calls, escape all control characters (`AscW < 32`) in `JsonEscape`, rebuild `word-template/EqnTide.dotm` via `word-template/build_dotm.py`, and verify with `python3 -m unittest discover -s tests/python -p test_word_ribbon.py`.
- [x] 3.2 Update `wps-addin/main.js`, `wps-addin/taskpane.js`, and WPS add-in packaging/installation scripts (`scripts/install-addin.sh`, `scripts/install-addin.ps1`, `scripts/install-session-addins.py`) so WPS requests include `X-EqnTide-Token`, and verify with `npm test --prefix tests/wps`.

## 4. Binary/Package Parser and Renderer Hardening

- [x] 4.1 Harden `CfbReader` in `src/ole.rs` (`mini_cutoff` upper bound, `DirItem.size` bounds against file/mini-stream lengths, DIFAT sector count cap, and FAT/DIFAT cycle detection) without altering `make_ole`, and add regression tests for crafted OOM/cyclic CFB inputs plus `make_ole` round-trip compatibility.
- [x] 4.2 Enforce per-entry and total decompressed byte caps in `Package::read` (`src/docx/package.rs`) and add a unit test verifying oversized compressed entries return an error cleanly.
- [x] 4.3 Validate finite positive bounded `font_pt` in `src/service.rs`, `src/render/emf.rs`, and `src/render/svg.rs`, and verify with unit tests for `NaN`, `0.0`, and extreme `font_pt` values.

## 5. Cross-Component Verification

- [x] 5.1 Run full Rust (`cargo test --workspace`), Python (`python3 -m unittest discover -s tests/python`), and WPS JS (`npm test --prefix tests/wps`) test suites to confirm all security checks and MathType OLE/EMF/OMML conversion tests pass.
