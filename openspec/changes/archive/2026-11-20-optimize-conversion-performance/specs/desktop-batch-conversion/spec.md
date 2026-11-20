# Spec Delta

## MODIFIED Requirements

### Requirement: Structured batch toolbar and consistent control states
The desktop **Batch DOCX** tab SHALL organize its top controls into an **Input** group (`Add DOCX files…`, `Add folder…`, `Clear list`) and an **Options & Actions** group (target selector, overwrite toggle, `Convert`, `Stop`), without a global `Open output folder` toolbar button, and SHALL disable unavailable actions with consistent visual opacity.

#### Scenario: Empty queue shows consistent disabled states
- **WHEN** the **Batch DOCX** tab has no queued `.docx` files
- **THEN** `Clear list`, `Convert`, and `Stop` are disabled with consistent disabled styling, while `Add DOCX files…` and `Add folder…` remain enabled

#### Scenario: Multiple additive additions across different locations
- **WHEN** the user invokes `Add DOCX files…` or `Add folder…` multiple times across different directories or drives
- **THEN** newly discovered `.docx` files are appended without duplicates alongside existing entries while retaining each folder item's own `source_root` for sibling output generation

#### Scenario: Active run enables stop and disables editing controls
- **WHEN** a batch conversion is running
- **THEN** `Stop` is enabled and `Add DOCX files…`, `Add folder…`, `Clear list`, target selection, overwrite toggle, and `Convert` are disabled until the run finishes

## ADDED Requirements

### Requirement: Isolated sequential-file batch worker with multi-core per-document conversion and cooperative stop
The desktop **Batch DOCX** runner SHALL process selected `.docx` files one file at a time inside an isolated headless worker subprocess (`--batch-worker`) that uses multi-core parallelism within each document and exits upon completion or cancellation so the operating system reclaims worker memory immediately, while honoring **`Stop`** cooperatively without corrupting source files or leaving temporary artifacts behind.

#### Scenario: Multiple queued files convert one-by-one with multi-core per-file throughput and reclaimable memory
- **WHEN** multiple `.docx` files are selected and the user clicks **Convert**
- **THEN** the worker subprocess processes each selected file sequentially (parallelizing equations and ZIP packaging inside each file), reports per-row progress and output metadata as each file finishes, and exits after completion so memory is returned to the operating system

#### Scenario: Clicking Stop halts remaining queued files safely
- **WHEN** a batch conversion is active and the user clicks **`Stop`**
- **THEN** the worker subprocess receives the cancellation signal, stops claiming unstarted files from the queue, discards any incomplete temporary output without modifying untouched source files, and exits cleanly so the UI marks unstarted rows stopped and re-enables editing controls
