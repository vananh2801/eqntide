# Tasks

## 1. Baseline RSS & Memory Benchmark Harness

- [x] 1.1 Extend `src/docx/perf_tests.rs` (or add a dedicated memory benchmark test) to generate a multi-thousand-equation `.docx` (both OMML and MathType inputs) and report per-phase runtime and resident memory (`Peak RSS` on Linux/macOS or allocation counters) as a reproducible Phase 1 baseline; verify with `cargo test --release --lib docx::perf_tests -- --ignored --nocapture`.

## 2. Zero-Allocation OOXML Namespaces & Prefixes in `src/xml.rs`

- [x] 2.1 Replace per-node heap `String` fields for `prefix`, `ns`, and `nsdecls` in `src/xml.rs` (`Element`, `Attr`, `NsDecl`) with `Cow<'static, str>` backed by `intern_ns` / `intern_prefix` helpers for standard OOXML/MathML/VML/OLE namespaces and prefixes while preserving existing `Element::new`, `attr`, `is`, and `set_attr` ergonomics; verify workspace compilation and all XML unit tests pass via `cargo test --workspace --lib xml`.
- [x] 2.2 Add a unit test in `src/xml.rs` asserting that parsed standard OOXML elements (`w:`, `m:`, `r:`, `v:`, `o:`, `mc:`) store `Cow::Borrowed` pointers without heap allocation for `ns` and `prefix`, and verify `cargo test --workspace`.

## 3. File-Backed Lazy ZIP & Streaming Output (`src/docx/package.rs`)

- [x] 3.1 Refactor `Package` in `src/docx/package.rs` to support both in-memory slices (`Package::read`) and file-backed lazy archives (`Package::open(path)` over `ZipArchive<BufReader<File>>` using `EntryData::{Loaded, Deferred, Removed}`), preserving `MAX_DOCX_ENTRY_BYTES` / `MAX_DOCX_TOTAL_BYTES` limits, last-entry-wins duplicate-name semantics, and transient non-caching reads via `load_xml()` and `load_transient_batch()` + `PartLookup`; verify with unit tests in `src/docx/package.rs` including oversized-entry rejection and lazy pass-through behavior.
- [x] 3.2 Implement `Package::write_to_file_atomic(output_path, cancel)` using a sibling temporary file (`0o600` on Unix) + `TempFileGuard` RAII cleanup guard, symlink resolution and permission preservation, and streamed bounded-batch compression (`STREAM_COMPRESS_BATCH = 64`: copying untouched entries via `raw_copy_file` without decompressing, compressing modified/new entries in small `rayon` batches directly into `ZipWriter<BufWriter<File>>`, syncing and closing the source archive before `std::fs::rename`); add tests in `src/docx/package.rs` verifying in-place (`input == output`) replacement, symlink & permission preservation, cancellation cleanup leaving no temp file, and roundtrip equality with `Package::write`.

## 4. Chunked Parallel Equation Processing & Early Buffer Reclamation

- [x] 4.1 Update `src/docx/pipeline.rs` and `src/docx/job.rs` (`OMML -> MathType`) to use `Package::open` / `write_to_file_atomic` in `convert_with_cancel`, borrow `&'a Element` per chunk (`EQUATION_CHUNK_SIZE = 128`) assembling `.bin`/`.emf`/`.png` and DOM nodes per chunk before advancing, and reset `doc_root` / `rels_root` immediately after serialization; add a multi-equation equivalence test verifying chunked output matches deterministic part ordering, `rId`s, untouched `Stored` compression preservation, and error isolation.
- [x] 4.2 Update `src/docx/to_omml.rs` (`MathType -> OMML`) to use `Package::open` / `write_to_file_atomic` in `convert_to_omml_with_cancel`, process items in bounded chunks (`TO_OMML_CHUNK_SIZE = 128`) with `ToOmmlWorkRef` + `extra_children` cross-chunk path-shift tracking, load `.bin`/`.emf`/`.wmf` payloads transiently per chunk via `load_transient_batch` + `PartLookup`, propagate `.rels` errors and mark unreferenced orphans `Removed` in `cleanup_unreferenced_files`, and drop finished part DOMs after serialization; verify with `cargo test --lib docx::to_omml`.
- [x] 4.3 Update `src/docx/upgrade.rs` (`Legacy -> Modern MathType`) to delegate `ModernOmml` to `convert_to_omml_with_cancel` and use `Package::open` / `write_to_file_atomic` in `upgrade_legacy_mathtype_docx_with_cancel`, process upgrade items in bounded chunks (`UPGRADE_CHUNK_SIZE = 128`) with `UpgradeWorkRef` + `load_transient_batch`, mark retired legacy media `Removed` via `cleanup_unreferenced_files`, and drop finished part DOMs after serialization; verify with `cargo test --lib docx::upgrade`.

## 5. Integration Equivalence & Memory Verification

- [x] 5.1 Run full workspace test suite (`cargo test --workspace`) and verify all CLI, desktop batch, security, and cross-conversion integration tests (`tests/conversion_matrix.rs`, `tests/cross_conversion.rs`, `tests/export_verification.rs`) pass without regressions.
- [x] 5.2 Re-run the Phase 1 memory/performance benchmark from Task 1.1 on a multi-thousand-equation `.docx` in both directions (OMML -> MathType and MathType -> OMML) and verify Peak RSS reduction (comparing `/usr/bin/time -v` Max RSS before vs. after) while maintaining multi-core throughput.
