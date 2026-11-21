# docx-parallel-conversion Specification

## Purpose

Defines multi-core parallel conversion behavior inside `.docx` documents and across CLI directory batches while preserving deterministic output, per-equation error isolation, and bounded resource usage.

## Requirements

### Requirement: Deterministic parallel equation conversion within a document
The `.docx` conversion pipelines (`OMML -> MathType`, `MathType -> OMML`, and `Legacy -> Modern MathType`) SHALL parallelize independent per-equation computation across available CPU cores while assembling the output package in deterministic document order.

#### Scenario: Multi-equation document produces deterministic output across parallel workers
- **WHEN** a `.docx` document containing multiple equations across `word/document.xml` and related parts is converted with parallel execution enabled
- **THEN** all convertible equations are processed, package part names (`oleObject*.bin`, `equation*.emf`/`equation*.png`) and relationship IDs (`rId*`) are assigned in deterministic document order, and the resulting document matches single-threaded semantics

#### Scenario: Per-equation error isolation is preserved under parallel execution
- **WHEN** one equation inside a `.docx` fails to decode, encode, or render while other equations succeed
- **THEN** only the failing equation is left untouched and counted in `errors`, while all other valid equations are converted and written to the output `.docx`

### Requirement: Bounded-memory CLI directory batch conversion with multi-core per-document parallelism
CLI directory conversion SHALL process discovered `.docx` files one file at a time (preventing unbounded peak RAM across large files) while parallelizing equation computation and ZIP compression inside each file up to the configured or default concurrency limit and reporting ordered summary and progress output.

#### Scenario: Directory batch converts files sequentially with multi-core per-file parallelism
- **WHEN** `eqntide-cli` converts a directory containing multiple `.docx` files
- **THEN** files are processed one at a time in deterministic order (using multi-core parallelism inside each file), preserving relative directory structure and reporting accurate aggregate `found`, `converted`, `skipped`, and `errors` counts

#### Scenario: User overrides worker concurrency via CLI
- **WHEN** the user specifies a concurrency limit (e.g., `--jobs N`) for `eqntide-cli`
- **THEN** conversion respects the requested worker bound within each file without exceeding it or failing on single-thread (`--jobs 1`) mode

### Requirement: Cooperative cancellation during document conversion
Document conversion entry points used by interactive batch workflows SHALL support cooperative cancellation checks so long-running conversions can abort cleanly before committing output.

#### Scenario: Cancelled conversion leaves no committed output
- **WHEN** cancellation is signaled before a `.docx` conversion finishes assembling and writing its output
- **THEN** the conversion aborts cleanly without committing partial output to the destination path

### Requirement: Bounded-memory chunked parallel equation processing within a document
The `.docx` conversion pipelines (`OMML -> MathType`, `MathType -> OMML`, and `Legacy -> Modern MathType`) SHALL process equations inside each XML part in bounded parallel chunks rather than materializing all prepared equation binaries, previews, and rewritten XML subtrees simultaneously, while producing deterministic output identical to unchunked execution.

#### Scenario: Large multi-thousand-equation document executes in bounded chunks
- **WHEN** a `.docx` document containing thousands of equations is converted using multi-core parallelism
- **THEN** equation preparation runs in bounded chunks with completed equation payloads assembled and intermediate per-chunk buffers released before advancing to the next chunk, preserving deterministic part names, relationship IDs, and summary counts

### Requirement: File-backed streaming ZIP packaging with atomic commit and untouched-entry pass-through
File-based `.docx` conversion entry points SHALL stream input archives directly from disk without buffering the entire input and output `.docx` files simultaneously in memory, SHALL pass through unmodified archive entries without decompressing and recompressing them, SHALL enforce existing per-entry and total decompressed size limits, and SHALL write output through a temporary file committed atomically so in-place conversions and cancelled runs never corrupt source files.

#### Scenario: In-place file conversion preserves source integrity on cancellation or failure
- **WHEN** a file-based `.docx` conversion targets the same path as the input file and is cancelled or encounters an I/O or archive error before completion
- **THEN** the incomplete temporary file is discarded and the original `.docx` file remains unmodified

#### Scenario: Unmodified media and non-equation parts pass through without full recompression
- **WHEN** a `.docx` document containing unmodified images, styles, or non-equation parts is converted via file-based entry points
- **THEN** unmodified archive entries are copied into the output package without holding full decompressed copies alongside all converted parts, while newly generated or modified parts are compressed and written deterministically
