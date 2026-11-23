# bridge-and-parser-security Specification

## Purpose

Defines security boundaries, authentication, path containment, and resource limits for the local HTTP bridge, desktop single-instance IPC, Office add-in clients, and document/binary parsers while preserving MathType OLE and OOXML output compatibility.

## Requirements

### Requirement: Local HTTP bridge requires host, origin, and token validation
The local HTTP bridge SHALL require every incoming request to include a `Host` header resolving to a local loopback address (`127.0.0.1`, `localhost`, or `::1`), SHALL reject untrusted or `null` `Origin` headers, SHALL require `https://` for non-loopback allowed origins, and SHALL require a valid per-user authentication token on all state-changing, session-reading, logging, and conversion endpoints (except `GET /health` and CORS `OPTIONS` preflight).

#### Scenario: Missing Host header or DNS rebinding Host is rejected
- **WHEN** a client sends an HTTP/1.0 request with no `Host` header or a request with `Host: attacker.example:17890`
- **THEN** the bridge MUST reject the request with HTTP 403

#### Scenario: Untrusted or null Origin is rejected
- **WHEN** a browser page or sandboxed iframe sends a request with `Origin: null` or `Origin: https://evil.example` or `Origin: http://office.com`
- **THEN** the bridge MUST reject the request with HTTP 403 and MUST NOT emit wildcard `Access-Control-Allow-Origin: *`

#### Scenario: Unauthenticated API request is rejected while synchronized clients succeed
- **WHEN** a client calls `/session/open`, `/session/:id`, `/session/:id/update`, `/convert/snippet`, `/convert/document`, or `/window/show` without a valid per-user token
- **THEN** the bridge MUST return HTTP 401
- **AND WHEN** the Desktop app, Word VBA template, or WPS JS Add-in supplies the current user's token
- **THEN** the bridge MUST accept the request and return normal conversion/session responses

### Requirement: Static add-in file serving enforces strict root containment
The bridge route serving WPS add-in static assets SHALL only serve regular files strictly contained inside the resolved add-in directory, rejecting path traversal segments, drive prefixes, UNC paths, absolute paths, and percent-encoded traversal sequences.

#### Scenario: Windows drive-letter or traversal path is rejected
- **WHEN** a client requests `/wps-addin/C:/Windows/win.ini`, `/wps-addin/C:\Users\secret.txt`, or `/wps-addin/%2e%2e/Cargo.toml`
- **THEN** the bridge MUST return HTTP 404 (or 403) and MUST NOT read any file outside the `wps-addin` directory

### Requirement: Bridge eliminates arbitrary launcher execution and unsafe temporary file writes
The bridge SHALL NOT expose an arbitrary OS launcher endpoint (`/open/folder`), SHALL NOT write incoming equation payloads to fixed shared paths such as `/tmp/last_open_req.xml`, and SHALL validate session existence before writing any per-session temporary `.emf` preview file.

#### Scenario: Removed launcher endpoint returns not found
- **WHEN** a client sends `POST /open/folder`
- **THEN** the bridge MUST return HTTP 404

#### Scenario: Session open does not create world-readable temp dumps
- **WHEN** a client calls `POST /session/open` with equation XML or LaTeX
- **THEN** the bridge MUST NOT write `/tmp/last_open_req.xml`

#### Scenario: Invalid session update does not write temp files
- **WHEN** a client calls `POST /session/nonexistent-id/update`
- **THEN** the bridge MUST reject the request with HTTP 404 without creating any `.emf` file in the system temporary directory

### Requirement: Document conversion endpoint restricts output targets
The `/convert/document` endpoint SHALL verify that both the input path and any specified output path have a `.docx` extension (case-insensitive) and resolve to regular writable document targets without corrupting non-DOCX files.

#### Scenario: Non-DOCX output path is rejected
- **WHEN** a client calls `POST /convert/document` with a `.docx` input and an `output_path` ending in `.exe`, `.bat`, or `.txt`
- **THEN** the bridge MUST reject the request with HTTP 400 without writing the output file

### Requirement: Bridge bounds request sizes, concurrency, wait timeouts, and stale state
The bridge SHALL enforce an upper bound on HTTP request body size, bounded request concurrency, a clamped maximum long-poll timeout, and periodic cleanup of expired sessions and temporary `.emf` files.

#### Scenario: Oversized HTTP body is rejected
- **WHEN** a client sends a request body exceeding the configured maximum size limit
- **THEN** the bridge MUST reject the request with HTTP 413 without allocating unbounded memory

#### Scenario: Stale sessions and temporary EMF previews are cleaned up
- **WHEN** sessions expire past the retention window or are cancelled/removed
- **THEN** the session manager MUST evict stale entries and delete associated temporary `.emf` files

### Requirement: Desktop single-instance IPC isolates lock and socket per user
The desktop single-instance mechanism SHALL store its lock, endpoint metadata, and IPC verification secret in the user's private local data directory with restrictive permissions rather than falling back to a shared world-writable temporary directory, and SHALL validate incoming IPC launch messages before dispatching them.

#### Scenario: Forged or unauthenticated local IPC message is ignored
- **WHEN** an untrusted local process writes a launch argument list to the desktop IPC socket without the user's IPC secret
- **THEN** the running desktop instance MUST reject or ignore the message

### Requirement: OLE CFB, DOCX ZIP, and layout/rendering pipelines enforce input bounds while preserving output compatibility
The OLE CFB reader SHALL validate header sector counts, mini-stream cutoffs, stream sizes, and FAT/DIFAT chain acyclicity; the DOCX package reader SHALL enforce per-entry and total decompressed size limits; and snippet/document entry points SHALL validate finite positive `font_pt` values. Generated OLE CFB, MTEF, EMF, and Flat OPC outputs for valid equations SHALL remain unchanged for Microsoft Word and MathType compatibility.

#### Scenario: Malformed OLE header cannot trigger OOM or infinite loop
- **WHEN** the OLE reader parses a compound file claiming an oversized `mini_cutoff`, stream size exceeding file bounds, or cyclic FAT/DIFAT chains
- **THEN** the reader MUST fail cleanly with `None`/error without allocating oversized buffers or hanging

#### Scenario: Compressed DOCX zip bomb is rejected
- **WHEN** the DOCX reader opens an archive whose uncompressed entry size or cumulative uncompressed size exceeds configured limits
- **THEN** the reader MUST return an error without exhausting system memory

#### Scenario: Generated MathType OLE equation remains readable by Word and MathType
- **WHEN** a valid equation is converted or inserted into Word via the updated VBA template or batch pipeline
- **THEN** the resulting OLE compound binary, embedded MTEF stream, and EMF preview MUST retain the exact MathType `Equation.DSMT4` structure

### Requirement: Bridge session synchronization recovers from poisoned locks without panicking
The local HTTP bridge session store SHALL tolerate poisoned synchronization locks without panicking or terminating active bridge request-handling threads, returning clean unavailable or error outcomes or recovering consistent session state across session lifecycle operations.

#### Scenario: Poisoned session lock does not crash the HTTP bridge
- **WHEN** a thread panics while holding an internal session synchronization lock and a subsequent client or editor invokes session open, lookup, update, cancel, wait, or stale-cleanup operations
- **THEN** the bridge MUST NOT panic across subsequent session requests and MUST continue serving valid HTTP responses or clean error states

### Requirement: XML and DOCX parsing dependencies comply with workspace security advisories and license policy
Workspace XML and OOXML container dependencies SHALL be free of unresolved security advisories and disallowed licenses under the repository's dependency audit checks while preserving bounded memory and linear-time attribute parsing guarantees.

#### Scenario: Dependency audit passes with adversarial XML and archive bounds intact
- **WHEN** the repository dependency security and license audit is executed and malformed XML start tags or oversized DOCX entries are parsed
- **THEN** the dependency audit MUST pass without unaddressed vulnerability failures and parsers MUST reject malformed or oversized inputs without panicking or quadratic CPU stalling
