# Design

## Context

See [proposal.md](./proposal.md) for motivation. Key codebase constraints discovered during exploration:
- Font loading in `src/render/font.rs` uses `static F: OnceLock<Fonts>`, so `&'static Fonts` is already `Send + Sync` and safe for lock-free concurrent reads across threads.
- `desktop/src/batch.rs` (`convert_item`) writes each document through a sibling `tempfile` before atomic `persist` / `persist_noclobber`, providing crash/cancellation safety per file.
- `src/docx/job.rs`, `src/docx/upgrade.rs`, and `src/docx/to_omml.rs` currently interleave pure per-equation CPU work with sequential DOM/ZIP mutations, and contain $O(N^2)$ ID/content-type scans plus per-equation XML/buffer allocations.
- Users have heterogeneous workloads (1 huge `.docx` vs. many small `.docx` files, CLI vs. Desktop, EMF vs. PNG vs. OMML), so optimizing only file-level or only equation-level would leave half of users unserved.

## Goals / Non-Goals

**Goals:**
- Measure baseline phase timings before optimizing so every optimization targets verified bottlenecks.
- Eliminate $O(N^2)$ algorithmic bottlenecks and hot-path heap allocations in `src/docx/` and `src/render/`.
- Unify file-level and equation-level parallelism on a single bounded thread pool to avoid thread oversubscription.
- Preserve deterministic `.docx` output (part indices, `rId`s, XML order) and per-equation error isolation.
- Rename Desktop Batch **`Stop after current file`** to **`Stop`** with safe cooperative cancellation across concurrent workers.

**Non-Goals:**
- Changing equation visual layout, MTEF binary compatibility, or OMML semantic output.
- GPU/wgpu compute offloading for equation previews (CPU vector/raster paths are already fast once parallelized and allocation-trimmed).
- Parallelizing single-snippet HTTP bridge requests (`ConvertSnippetRequest`), which are already sub-millisecond single-equation calls.

## Decisions

### 1. Phased Execution: Benchmark First, Then Single-Thread Hotspots, Then Parallelism
- **Decision**: Execute in 4 stages: (1) Benchmark/Profiling baseline -> (2) Single-thread $O(N^2)$ & allocation fixes -> (3) Shared-pool multi-core parallelism -> (4) Conditional ZIP packaging optimization based on stage 1 data.
- **Rationale**: Fixing $O(N^2)$ scans and repeated allocations improves single-core latency directly and multiplies with thread count, while profiling prevents premature complexity in ZIP handling if ZIP I/O turns out minor.

### 2. Shared Global Thread Pool (`rayon`) vs. Ad-hoc `std::thread::spawn`
- **Decision**: Prefer a shared work-stealing pool (`rayon` with configurable pool size / CLI `--jobs` override) over nested `std::thread::spawn` loops.
- **Rationale**: Nested file-level + equation-level `std::thread` pools risk $N \times N$ oversubscription (e.g., 64+ threads thrashing on 8 cores). A single work-stealing pool naturally saturates cores whether converting 1 file with 1,000 equations or 100 files with 10 equations each.
- **Alternative considered**: Hand-rolled `std::thread::scope` worker queue. Rejected for nested file+equation workloads because work-stealing between outer files and inner equations is non-trivial to maintain cleanly without `rayon`.

### 3. Two-Phase Equation Pipeline (Parallel Compute -> Sequential Assemble)
- **Decision**: Refactor `pipeline.rs`/`job.rs`, `upgrade.rs`, and `to_omml.rs` into:
  1. **Collect**: Walk document XML parts once to gather equation work items (`eq_num`, source node/bytes, target slot).
  2. **Parallel Compute**: Map each work item independently (`omml_to_mathml`, `mathml_to_mtef_bytes`, `make_ole`, `render_emf_with_mtef`/`render_preview`, or `ole_to_mathml` -> `mathml_to_omml`) into `Vec<Result<PreparedOutcome>>`.
  3. **Sequential Assemble**: Fold outcomes in `eq_num` order to allocate `oleObject*.bin`, `equation*.emf/png`, `rId*`, update `[Content_Types].xml`, and patch DOM nodes.
- **Rationale**: Guarantees identical deterministic part numbering and error isolation without mutex contention inside XML/Package trees.

### 4. Hot-Path Single-Thread Fixes
- **Monotonic ID & Content-Type Tracking**: Replace `while existing_ids.contains(...)` from `1` (`job.rs::new_rel_id`, `upgrade.rs::next_unique_id`) and linear `ct_root.elements().any(...)` (`package.rs::ensure_content_type`) with $O(1)$ counters and `HashSet<String>` caches.
- **Cached Static XML & Move Semantics**: Cache `ole_xml::shapetype()` via `OnceLock<Element>` instead of parsing XML per equation; consume `Picture.data` by value instead of cloning.
- **Streaming Buffers in Renderers**: Write EMF record integers directly into `Writer::buf` (`render/emf.rs`) to eliminate per-segment `Vec<u8>` allocations; in `render/png.rs`, avoid allocating 5 candidate row `Vec<u8>` buffers per scanline and evaluate a faster zlib `Compression` level (balancing speed vs. size).

### 5. Cooperative `Stop` Semantics in Desktop Batch & Core
- **Decision**: Rename `"Stop after current file"` to `"Stop"` in `desktop/src/app.rs` (keeping the close-guard `"Stop batch"` dialog consistent). Pass `stop: &AtomicBool` (or cancellation token) to batch workers and optionally check it between equation compute chunks inside `convert_*` functions.
- **Rationale**: With parallel workers, "after current file" is ambiguous because multiple files run simultaneously. Because `desktop/src/batch.rs::convert_item` writes into a `tempfile` and only commits via `persist` on success, aborting mid-document cleanly drops temporary data without harming source `.docx` files.

### 6. Conditional ZIP Packaging Optimization (Stage 4)
- **Decision**: Only optimize `Package::write` in `src/docx/package.rs` if Stage 1 profiling shows ZIP compression takes a significant share (e.g., >15-20% of runtime). Options to evaluate then: parallel per-entry deflate compression before central-directory assembly, or pass-through copying of unchanged compressed entries.

## Risks / Trade-offs

- **Memory footprint under high concurrency** -> A batch of very large `.docx` files processed simultaneously holds multiple uncompressed `Package`s in RAM. Mitigation: bound file-level concurrency or size-gate large files; keep compute buffers scoped per equation.
- **Interleaved CLI logs / progress** -> Concurrent CLI workers printing per-file status could interleave stdout/stderr lines. Mitigation: emit buffered/synchronized per-file completion summaries via channel or mutex-guarded writer.
- **PNG size vs. speed trade-off** -> Lowering PNG zlib compression or simplifying row filter selection may slightly increase `.png` preview byte size. Mitigation: benchmark both speed gain and byte delta on representative equations before finalizing PNG settings.
- **UI test coupling** -> Existing tests in `desktop/src/app.rs` assert `"Stop after current file"`. Mitigation: update assertions alongside the label change and add concurrent batch + cancellation tests.
