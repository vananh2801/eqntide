# Tasks

## 1. Baseline Benchmarking & Profiling

- [x] 1.1 Add a reproducible conversion benchmark/timing harness covering single large `.docx` (~500+ equations) and multi-file batch workloads across `OMML -> MathType (EMF/PNG)`, `MathType -> OMML`, and `Upgrade -> MathType`, and record phase timing breakdowns (`ZIP read`, `XML parse`, `compute/render`, `XML serialize`, `ZIP write`) via `cargo test --release` or `cargo bench`.

## 2. Single-Thread & Allocation Hotspot Elimination

- [x] 2.1 Replace $O(N^2)$ scans in `src/docx/job.rs` (`new_rel_id`), `src/docx/upgrade.rs` (`next_unique_id`), and `src/docx/package.rs` (`ensure_content_type`) with $O(1)$ counters/`HashSet` tracking, and verify existing `docx` unit tests pass.
- [x] 2.2 Cache `ole_xml::shapetype()` via `OnceLock<Element>` in `src/docx/ole_xml.rs` and consume `Picture.data` without cloning in `src/docx/job.rs`, verifying with `cargo test --lib docx`.
- [x] 2.3 Eliminate per-record `Vec<u8>` allocations in `src/render/emf.rs` (`Writer` / `write_segs`) and reduce per-row allocations and excessive deflate cost in `src/render/png.rs` (`encode_alpha`), verifying EMF/PNG output correctness and size/speed deltas with `cargo test --lib render` and the benchmark from 1.1.

## 3. Parallel Equation Conversion Inside `.docx`

- [x] 3.1 Add `rayon` (or shared bounded pool) to `Cargo.toml` and refactor `src/docx/pipeline.rs` + `src/docx/job.rs` (`OMML -> MathType`) into parallel compute and deterministic sequential assembly, adding tests verifying identical output ordering and per-equation error isolation.
- [x] 3.2 Refactor `src/docx/upgrade.rs` (`Legacy -> Modern MathType`) and `src/docx/to_omml.rs` (`MathType -> OMML`) into parallel compute and deterministic sequential assembly, verifying with `cargo test --lib docx` and `tests/cross_conversion.rs`.
- [x] 3.3 Add cooperative cancellation (`AtomicBool`) support to document conversion entry points so aborted runs exit before committing output, and verify with a unit test confirming no destination file is written on cancellation.

## 4. Parallel Batch Execution & Desktop `Stop` UX

- [x] 4.1 Update `src/cli/dispatch.rs` (`run_directory`) to process `.docx` files one at a time (keeping peak RAM bounded to 1 document) while running multi-core parallelism inside each file via an optional `--jobs` CLI argument in `src/cli.rs`, verifying CLI single-thread (`--jobs 1`) and multi-thread directory runs pass `cargo test`.
- [x] 4.2 Run Desktop batch conversion (`desktop/src/batch.rs`) one file at a time inside a self-spawned `--batch-worker` subprocess (`desktop/src/main.rs`) with multi-core equation/ZIP parallelism inside each file, cooperative `Stop` handling (preserving `tempfile` safety), and full OS memory reclamation on exit; add unit tests for sequential multi-file execution, stopping, and the worker stdio protocol.
- [x] 4.3 Rename the Desktop Batch DOCX **`Stop after current file`** button to **`Stop`** in `desktop/src/app.rs` and update UI/state tests asserting the button label, disabled styling, and active-run state.

## 5. Conditional ZIP Packaging & Verification

- [x] 5.1 Evaluate `Package::write` (`src/docx/package.rs`) against post-Stage-3 benchmark numbers; if ZIP compression exceeds ~15% of total runtime, parallelize entry compression or copy unchanged compressed entries verbatim and verify round-trip `.docx` validity with `cargo test`.
- [x] 5.2 Run the full workspace test suite (`cargo test --workspace`) and re-run the Stage 1 benchmark suite to verify end-to-end correctness and speedup across single-file and multi-file workloads.
