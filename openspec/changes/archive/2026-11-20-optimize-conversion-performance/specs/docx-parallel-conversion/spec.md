# Spec Delta

## Purpose

Defines multi-core parallel conversion behavior inside `.docx` documents and across CLI directory batches while preserving deterministic output, per-equation error isolation, and bounded resource usage.

## ADDED Requirements

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
