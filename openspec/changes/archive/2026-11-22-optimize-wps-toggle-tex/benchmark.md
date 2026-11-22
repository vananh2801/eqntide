# WPS Toggle TeX benchmark

## Setup

- WPS Writer for Linux 12.1.2.29182 and EqnTide bridge 0.7.0.
- Baseline WPS add-in from `049eca1` with aggregate phase timing added. The final candidate imports verified LaTeX replacements on Linux with MathType dimensions, alternative text, and baseline encoded in the Flat OPC package.
- Each run starts from a fresh copy. Insertion wall time includes the yields between groups. Bridge time covers the snippet queue and is reported separately.

## Pilot document with 256 formulas

| Build | Insertion runs (s) | Median insertion (s) | Median bridge (s) | Converted / skipped |
| --- | --- | ---: | ---: | ---: |
| Baseline | 12.313, 13.401, 12.725 | 12.725 | 2.822 | 254 / 2 |
| Intermediate candidate | 7.111, 7.505, 7.172 | 7.172 | 2.693 | 254 / 2 |

Median insertion time fell by **43.6%**. The saved outputs agreed on paragraph text, bookmark counts, the 254 MathType native equation streams, EMF bytes, dimensions, and equation baseline values. For 59 paragraph marks the baseline file serialized an explicit zero position where the candidate left the default value implicit.

Trials with display updates disabled took 8.277 s and 7.026 s. Background pagination already reported disabled. The candidate leaves these WPS settings alone.

## Reference document with 4,336 formulas

The original live run recorded 19m 27s from the first insertion log to completion, with 4,334 converted and 2 skipped. This is one historical observation.

Profiling the last 120 formulas in this document isolated formatting as the dominant cost. The baseline insertion runs took 59.565, 59.445, and 59.823 s. About 58 s per run was spent in formatting. A direct-import trial with the regular WPS formatting calls still took 59.123 s. Encoding the formatting in the imported package reduced the same subset to 0.726–0.744 s.

| Final candidate run | Preparation (s) | Bridge (s) | Insertion (s) | Converted / skipped |
| --- | ---: | ---: | ---: | ---: |
| 1 | 1.504 | 46.257 | 72.238 | 4,334 / 2 |
| 2 | 0.849 | 46.111 | 39.363 | 4,334 / 2 |
| 3 | 0.759 | 44.381 | 41.325 | 4,334 / 2 |
| Median | 0.849 | 46.111 | 41.325 | 4,334 / 2 |

Median insertion was 96.5% lower than the historical 19m 27s observation.

| Instrumented baseline run | Preparation (s) | Bridge (s) | Insertion (s) | Converted / skipped |
| --- | ---: | ---: | ---: | ---: |
| 1 | 2.613 | 46.844 | 1,720.550 | 4,334 / 2 |
| 2 | 1.217 | 45.977 | 1,971.530 | 4,334 / 2 |
| 3 | 1.298 | 46.131 | 1,525.626 | 4,334 / 2 |
| Median | 1.298 | 46.131 | 1,720.550 | 4,334 / 2 |

The candidate reduced median insertion by **97.6%** across three runs per build.

All three candidate outputs matched the historical output on paragraph text, surrounding paragraph and run properties after normalizing explicit zero baseline values, and counts of paragraphs (3,310), tables (46), cells (404), bookmarks (59), and OLE objects (4,338 including four existing objects). All MathType native equation streams and EMF bytes matched in document order. Eleven equation baselines were 1 pt lower. The largest dimension difference was 1.45 pt on a 419.25 pt-wide equation. The largest relative dimension difference was 1.4% (0.1 pt).
