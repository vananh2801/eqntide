# Design

## Context

See `proposal.md` for motivation. Currently the local HTTP bridge (`127.0.0.1:17890`) serves both the native Desktop UI (`desktop/src/runtime.rs`), the Word VBA global template (`word-template/EqnTide.bas` packaged into `EqnTide.dotm`), and the WPS Office JS Add-in (`wps-addin/main.js`, `wps-addin/taskpane.js`). Because the project has not been released yet, breaking protocol updates can be rolled out synchronously across all four components.

## Goals / Non-Goals

**Goals:**
- Prevent unauthenticated local/cross-origin access, Windows path traversal, arbitrary file overwrites, and unsafe `/tmp` writes on the bridge and desktop IPC.
- Eliminate OOM/CPU DoS vectors in the HTTP bridge, OLE CFB reader, DOCX ZIP reader, and equation renderers.
- Keep `make_ole`, MTEF encoding, EMF generation, and Flat OPC XML output formats identical for Word/MathType compatibility.

**Non-Goals:**
- Replacing the HTTP transport on `127.0.0.1:17890` with Unix domain sockets / named pipes for WPS JS (WPS JS Add-in relies on `XMLHttpRequest` to localhost).
- Preserving backwards compatibility with unauthenticated copies of `EqnTide.dotm` or `wps-addin`.

## Decisions

### 1. Per-User Bridge Authentication Token (`X-EqnTide-Token`)
- **Decision**: When the bridge server starts (`eqntide-cli --server` or `eqntide-desktop`), generate or load a per-user cryptographically random token (UUID v4 or 32-byte hex) and store it in the user's local data directory (e.g. `ProjectDirs::from("com", "eqntide", "EqnTide").data_local_dir()/bridge.token`) with owner-only permissions (`0600` on Unix; user-only directory on Windows).
- **Client Integration**:
  - `desktop/src/runtime.rs`: Reads the token from the per-user state file (or shared in-process state when hosting the bridge) and attaches `X-EqnTide-Token` to all bridge requests.
  - `word-template/EqnTide.bas`: Reads `%LOCALAPPDATA%\eqntide\EqnTide\bridge.token` via `Scripting.FileSystemObject` / `ADODB.Stream` and sets `http.setRequestHeader "X-EqnTide-Token", token` (plus `JsonEscape` updated to escape all `AscW < 32` control characters).
  - `wps-addin/main.js` & `taskpane.js`: During implementation, verify whether WPS JS runtime exposes local file reading (`wps.FileSystem` / `OAAssist`) across Windows and Linux builds. If WPS JS cannot read the per-user token file directly in all target builds, provision a per-user config/token JS snippet during add-in installation (`install-addin.sh` / `install-addin.ps1` / `install-session-addins.py`) or serve an unguessable per-session token only via a restricted local mechanism, while still requiring the `X-EqnTide-Token` custom header (which triggers a strict CORS preflight and cannot be sent as a simple request by third-party pages).
- **Why**: Custom header + per-user secret defeats both CSRF from browsers and unauthorized local processes on multi-user systems.
- **Alternative Considered**: Requiring mTLS or moving WPS JS off HTTP. Rejected because VBA (`MSXML2.ServerXMLHTTP`) and WPS JS (`XMLHttpRequest`) cannot speak Unix domain sockets or easily manage client certificates on localhost.

### 2. Strict `Host`, `Origin`, and CORS Policy
- **Decision**:
  - Require `Host` header to be present and satisfy `is_valid_host` (rejecting HTTP/1.0 requests lacking `Host` with `403`).
  - Remove `""` and `"null"` from `is_allowed_origin` (`Origin` is optional for non-browser native clients like Word VBA/Desktop `ureq`, which omit `Origin`, and must still pass `X-EqnTide-Token`; when `Origin` IS present, `"null"` is rejected).
  - Restrict remote Office/WPS origins (`*.office.com`, `*.office365.com`, `*.microsoft.com`, `*.live.com`, `*.wps.cn`, `*.wps.com`, `*.kdocs.cn`) to `https://` only; allow `http://` only for loopback hosts (`127.0.0.1`, `localhost`, `::1`), plus `wps-addon:` / `file://` if required by WPS WebView.
  - Only include `Access-Control-Allow-Origin` when a validated `Origin` header was present (never fall back to `*`).
  - Include `X-EqnTide-Token` in `Access-Control-Allow-Headers`.

### 3. Path Containment, Endpoint Removal, and Temp File Hygiene
- **Decision**:
  - `serve_addin_file` (`src/bridge/server.rs`): Percent-decode or reject `%` traversal, reject any path where `Path::new(rel_path)` has components other than `Component::Normal` (this automatically blocks `C:`, `\\server\share`, `/`, `\`, and `..` on all platforms), and verify `fs::canonicalize(&candidate)?.starts_with(&fs::canonicalize(&base_dir)?)`.
  - Remove `/open/folder` from `src/bridge/server.rs` (confirmed unused by `desktop/`, `wps-addin/`, and `word-template/EqnTide.bas`; Desktop uses `runtime::open_folder` directly).
  - Remove `/tmp/last_open_req.xml` write from `handle_session_open`.
  - `/convert/document`: Enforce `.docx` extension on `output_path` as well as `input_path`, and reject paths containing null bytes or invalid targets.
  - `handle_session_update`: Validate `session_id` exists in `SessionManager` (and is a valid UUID) *before* rendering or writing `eqntide_{session_id}.emf`. Add periodic `cleanup_stale` invocation (e.g. on session open/sweep) that also removes stale `eqntide_*.emf` files.

### 4. Bridge Resource Bounds
- **Decision**:
  - Cap `read_body` at a safe maximum (e.g., 16 MiB) using `Read::take`, returning `413 Payload Too Large` on overflow.
  - Clamp `/session/:id/wait?timeout=` to `1..=300` seconds.
  - Bound concurrent HTTP worker threads (bounded worker pool or semaphore) so slowloris/long-poll floods cannot exhaust OS threads.

### 5. Single-Instance IPC Hardening (`desktop/src/instance.rs`)
- **Decision**:
  - Remove the shared `std::env::temp_dir().join("EqnTide")` fallback (fail cleanly with an actionable error if `ProjectDirs` cannot be created, or restrict directory mode to `0700` on Unix).
  - Do not unlink `desktop.lock` blindly on `PermissionDenied` if owned by another user; verify ownership/permissions where supported.
  - Authenticate IPC messages using a random per-user `ipc.secret` stored alongside `desktop.lock` (`{ "secret": "...", "args": [...] }`), rejecting messages without matching secret.

### 6. Parser & Renderer Hardening (`src/ole.rs`, `src/docx/package.rs`, `src/service.rs`, `src/render/`)
- **Decision**:
  - `src/ole.rs`:
    - Validate `mini_cutoff <= 65536` (and `<= data.len()` or standard `4096` tolerance) in `CfbHeader::parse`.
    - Cap `DirItem.size` in `read_mini_chain` / `read_regular_stream` against `data.len()` and `mini_stream.len()` before calling `Vec::with_capacity`.
    - Cap `collect_fat_sector_ids` DIFAT chain traversal by total possible sectors (`data.len() / header.sector_size + 1`) and track visited DIFAT sectors to prevent cycles.
    - Track visited sectors / bound `steps` in `read_chain` and `read_mini_chain` by `data.len() / sector_size + 1`.
  - `src/docx/package.rs`:
    - Enforce per-entry uncompressed cap and total archive decompressed cap (e.g., 64 MiB per part / 256 MiB total) in `Package::read` using `Read::take` rather than trusting ZIP header sizes alone.
  - `src/service.rs` & `src/render/`:
    - Validate `font_pt.is_finite() && (1.0..=200.0).contains(&font_pt)` at service/bridge entry points and in `render_emf_with_mtef` / `render_svg`.

## Risks / Trade-offs

- **WPS JS Add-in token access** → Need to verify during implementation whether WPS Office JS runtime on both Windows and Linux supports reading the per-user token file (via WPS FileSystem API) or requires writing a per-user `bridge-config.js` (with `0600` permissions) in the user's add-in directory when installing/starting the bridge.
- **Large legitimate `.docx` files** → Setting the DOCX ZIP decompression cap too low could reject large textbooks; choose generous limits (e.g., 256 MiB total, 64 MiB per XML/media part) that comfortably exceed real documents while blocking multi-GB zip bombs.
- **Word VBA / WPS Add-in synchronized rebuild** → `word-template/EqnTide.dotm` must be rebuilt via `word-template/build_dotm.py` and tested with `tests/python/test_word_ribbon.py` whenever `EqnTide.bas` changes.
