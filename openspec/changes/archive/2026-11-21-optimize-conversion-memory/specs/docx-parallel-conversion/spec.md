## ADDED Requirements

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
