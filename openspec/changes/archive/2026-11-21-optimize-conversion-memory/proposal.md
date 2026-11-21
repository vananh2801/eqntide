# Proposal

## Why

Following recent multi-threading optimizations, converting a single `.docx` document containing several thousand equations (either OMML → MathType or MathType → OMML / upgrade) can still spike process RSS to ~500 MB on Linux. This happens because the document pipeline holds the entire compressed input file, full uncompressed ZIP package entries, full XML DOM trees with heap-allocated namespace/prefix strings, all equation work items and prepared binary/EMF/XML outcomes simultaneously in memory, and duplicate ZIP write buffers on heap at once.

Phase 1 reduces peak RAM dramatically across Windows, macOS, and Linux without rewriting the XML DOM model or risking XML/Word compatibility regressions.

## What Changes

- **Chunked parallel equation execution**: Process equations within each XML part in bounded chunks (128 equations per batch) in `src/docx/job.rs`, `src/docx/to_omml.rs`, and `src/docx/upgrade.rs` rather than cloning every equation node and collecting all `PreparedOutcome` / `ToOmmlPrepared` / `UpgradePrepared` results across the entire document at once; load `.bin`/`.emf`/`.wmf` entries transiently per chunk (instead of caching them in `Package`), mark unreferenced entries as `Removed`, and drop finished XML DOM parts immediately after serialization.
- **File-backed streaming ZIP I/O with atomic commit**: Update `src/docx/package.rs` and file-based conversion entry points (`convert_with_cancel`, `convert_to_omml_with_cancel`, `upgrade_legacy_mathtype_docx_with_cancel`) so disk conversions read lazily from `BufReader<File>` (pass-through `raw_copy_file` for untouched media/parts instead of decompressing and recompressing everything into RAM) and stream compressed ZIP output through a sibling temporary file before atomic rename (preserving symlinks and permissions), eliminating simultaneous `input` + `Package` + `chunks` + `output` buffers while keeping in-place overwrites and cancellation crash-safe.
- **Zero-allocation OOXML namespace & prefix storage in `src/xml.rs`**: Replace per-node heap `String` copies of repeated namespace URIs and prefixes (`Element::prefix`, `Element::ns`, `Element::nsdecls`, `Attr::prefix`, `Attr::ns`) with `Cow<'static, str>` interning for standard OOXML/MathML/VML/OLE namespaces and prefixes, cutting DOM memory footprint and multi-thread allocator fragmentation across all platforms.
- **Phase 2 explicitly deferred**: Event-based streaming XML rewriting of `word/document.xml` (removing the per-part XML DOM entirely) is intentionally out of scope for Phase 1 so all existing XML rules, `sanitize_document_baselines`, and Word/WPS/MathType compatibility remain unchanged and byte-equivalent verifiable.

## Capabilities

### New Capabilities
<!-- None: Phase 1 strengthens bounded-memory guarantees within existing parallel conversion behavior. -->

### Modified Capabilities
- `docx-parallel-conversion`: Add per-document bounded-memory execution requirements (chunked parallel equation batches, file-backed streaming ZIP packaging with untouched-entry pass-through and atomic temp-file commit, and deterministic equivalence with unchunked output).

## Impact

- Affected code: `src/xml.rs`, `src/docx/package.rs`, `src/docx/pipeline.rs`, `src/docx/job.rs`, `src/docx/to_omml.rs`, `src/docx/upgrade.rs`, and `src/docx/perf_tests.rs` (plus minor `Cow` adjustments at `xml::Element` callsites if public fields change, or keeping compatible helper constructors).
- Public APIs (`convert`, `convert_bytes`, `convert_to_omml`, `upgrade_legacy_mathtype_docx`, `bridge`, `desktop`) remain backward compatible.
- Cross-platform: 100% pure Rust; benefits Linux, macOS, and Windows identically without platform-specific allocators.
