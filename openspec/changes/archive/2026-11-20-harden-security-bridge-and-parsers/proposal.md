# Proposal

## Why

A security audit of the EqnTide codebase uncovered multiple high- and medium-severity vulnerabilities across the local HTTP bridge, single-instance IPC, Word VBA/WPS add-in integration, and binary/package parsers (Windows drive-letter path traversal on `/wps-addin/`, unauthenticated bridge endpoints with `Origin: null` bypass, arbitrary file overwrite via `/convert/document`, arbitrary launcher execution via `/open/folder`, predictable `/tmp/last_open_req.xml` writes, and OOM/DoS vectors in the OLE CFB and DOCX ZIP readers). Because the project has not been released yet, all components can be updated together now to enforce strong security boundaries without preserving backwards compatibility with unauthenticated add-in templates.

## What Changes

- **BREAKING**: Require a per-user authentication token (`X-EqnTide-Token`) on mutating and sensitive local HTTP bridge endpoints, and update the Desktop client, Word VBA template (`EqnTide.bas` / `EqnTide.dotm`), and WPS JS Add-in (`wps-addin/`) synchronously to supply it.
- Reject missing/invalid `Host` headers (including HTTP/1.0 requests without `Host`), remove `Origin: null` from the CORS allowlist, and require `https://` for remote Office/WPS origins.
- Fix Windows drive-prefix path traversal in `/wps-addin/` static file serving using canonicalized containment checks and normal-component validation.
- Remove the unused `/open/folder` bridge endpoint and remove the hardcoded `/tmp/last_open_req.xml` debug file write.
- Validate `output_path` and path canonicalization on `/convert/document`, validate `session_id` before writing temporary `.emf` files, enforce HTTP request body/thread/timeout bounds, and wire up `SessionManager::cleanup_stale` plus temporary `.emf` cleanup.
- Harden `desktop/src/instance.rs` single-instance locking and IPC by removing unprotected `std::env::temp_dir()` fallbacks and authenticating local IPC payloads.
- Add strict bounds and cycle detection to the OLE CFB reader (`src/ole.rs`), decompression size caps to the DOCX ZIP reader (`src/docx/package.rs`), finite/range validation for `font_pt` in rendering/service entry points, and full control-character escaping in Word VBA `JsonEscape`.
- Preserve 100% of the generated OLE CFB (`make_ole`), MTEF, EMF, and Flat OPC XML output formats so Microsoft Word and MathType continue to open and edit generated equations identically.

## Capabilities

### New Capabilities
- `bridge-and-parser-security`: Authentication, origin/host validation, path containment, resource bounds, and temp/IPC isolation for the local HTTP bridge, desktop single-instance IPC, Office add-in clients, and OLE/DOCX parsers.

### Modified Capabilities
<!-- No existing capability requirements are changing; existing conversion, rendering, and editing output formats remain unchanged. -->

## Impact

- **Bridge & Core Services**: `src/bridge/server.rs`, `src/bridge/session.rs`, `src/service.rs`, `src/ole.rs`, `src/docx/package.rs`, `src/render/emf.rs`, `src/render/svg.rs`
- **Desktop Application**: `desktop/src/instance.rs`, `desktop/src/runtime.rs`
- **Office & WPS Clients**: `word-template/EqnTide.bas`, rebuilt `word-template/EqnTide.dotm`, `wps-addin/main.js`, `wps-addin/taskpane.js`, and installer/packaging scripts if token provisioning is required
- **Compatibility**: Pre-existing `EqnTide.dotm` or `wps-addin` copies without token support will receive `401 Unauthorized` from the hardened bridge until updated synchronously. Output `.docx` files and MathType OLE/EMF payloads remain fully compatible.
