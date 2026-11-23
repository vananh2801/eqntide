# Tasks

## 1. WPS ribbon Toggle TeX

- [x] 1.1 Replace the separate conversion and replacement phases in both directions with bounded dispatch and ordered writes. Add scheduler coverage for early replacement, out-of-order completions, the combined eight-item limit and payload release (`node tests/wps/test_wps_insertion.js`).
- [x] 1.2 Verify in WPS that progress reflects both operations, clears on exit, and yields during large runs. Exercise unsupported formulas, changed targets, ordinary illustrations and recovery from failed replacements. Check source preservation and final counts.

## 2. Live integration and performance

- [x] 2.1 Benchmark baseline and candidate for both directions on WPS Writer using fresh copies of the same large document. Record time to first replacement, total time, bridge and write timings, peak buffered items and peak process memory.
- [x] 2.2 Compare saved outputs with baseline for formula order, equation counts, editable MathType objects, surrounding formatting and document structure. Include mixed OMML and MathType, tables and adjacent formulas.
