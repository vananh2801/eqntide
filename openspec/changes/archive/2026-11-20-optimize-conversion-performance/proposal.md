# Proposal

## Why

Document and batch `.docx` conversions in EqnTide currently execute entirely on a single CPU thread and incur $O(N^2)$ scans and redundant allocations per equation. Because users have varied workloads—from converting a single large `.docx` containing hundreds or thousands of equations to batch-converting entire folders via the Desktop app or CLI—conversion throughput leaves multi-core CPUs underutilized and degrades on equation-dense documents.

## What Changes

- **Benchmarking & Profiling Baseline**: Establish reproducible micro-benchmarks/profiling across single-large-document and multi-file batch workloads (covering `OMML -> MathType EMF`, `OMML -> MathType PNG`, `MathType -> OMML`, and `Upgrade -> MathType`) to measure phase breakdowns (`ZIP read`, `XML parse`, `equation compute/render`, `XML serialize`, `ZIP write`) and verify speedups.
- **Single-Thread & $O(N^2)$ Hotspot Elimination**: Replace quadratic `rId` and `[Content_Types].xml` lookups with $O(1)$ counters/sets, cache static XML templates (`shapetype`), avoid unnecessary buffer clones, stream EMF binary records without per-command `Vec` allocations, and optimize PNG preview filtering/compression.
- **Unified Multi-Core Parallelism**:
  - **Equation-level parallelism**: Split `.docx` conversion pipelines (`OMML -> MathType`, `Upgrade -> MathType`, `MathType -> OMML`) into parallel pure-CPU compute and deterministic sequential XML/package assembly.
  - **File-level parallelism**: Run Desktop Batch DOCX and CLI directory conversion concurrently across a shared bounded thread pool (with optional CLI `--jobs` control and synchronized output reporting).
- **Desktop Batch Stop UX Update**: Rename the Desktop Batch DOCX **`Stop after current file`** button to **`Stop`** and support cooperative cancellation across parallel batch workers (halting new queue pickups and safely discarding incomplete temporary outputs without corrupting source files).
- **ZIP Packaging Optimization (conditional on profiling)**: Avoid re-compressing unchanged `.docx` entries or parallelize entry compression when ZIP I/O is a measurable bottleneck.

## Capabilities

### New Capabilities
- `docx-parallel-conversion`: Multi-core equation-level and CLI directory conversion that preserves deterministic equation ordering, per-equation error isolation, and cooperative cancellation.

### Modified Capabilities
- `desktop-batch-conversion`: Rename the toolbar stop action from `Stop after current file` to `Stop` and support concurrent batch file execution with cooperative stop semantics.

## Impact

- **Core library (`src/docx/`, `src/render/`)**: `pipeline.rs`, `job.rs`, `upgrade.rs`, `to_omml.rs`, `package.rs`, `ole_xml.rs`, `render/emf.rs`, `render/png.rs` (plus optional `rayon` dependency in `Cargo.toml` or a shared worker pool).
- **CLI (`src/cli/`, `src/cli/dispatch.rs`)**: Parallel directory batch execution, optional `--jobs` flag, ordered/non-interleaved progress output.
- **Desktop (`desktop/src/batch.rs`, `desktop/src/app.rs`)**: Concurrent `run_items` execution, `Stop` button label and status text, and updated UI/batch tests.
- **Specs & Tests**: Delta spec for `desktop-batch-conversion`, new spec for `docx-parallel-conversion`, determinism tests, and performance benchmarks.
