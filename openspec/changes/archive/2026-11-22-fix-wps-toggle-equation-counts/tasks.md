# Tasks

## 1. Confirm and fix classification

- [x] 1.1 Reproduce on a fresh copy of the reference DOCX in WPS and verify the excess targets by correlating candidate type and bridge outcome.
- [x] 1.2 Classify ambiguous targets using candidate-specific equation evidence. Test ordinary DrawingML and VML images, MathType OLE, OMML, and supported embedded previews.
- [x] 1.3 Verify the summary distinguishes excluded illustrations from confirmed equations that failed conversion, and run the WPS regression suite.

## 2. Verify on WPS

- [x] 2.1 Toggle the reference DOCX on WPS and verify 4,338 found, 4,338 converted, zero skipped or failed, and preserved ordinary illustrations.

## 3. Require an equation selection for Toggle LaTeX

- [x] 3.1 Disable adjacent object discovery for Toggle LaTeX and add automated regression coverage for a caret on either side of an equation or illustration. Run the WPS regression suite with the existing clicked-object and selected-range cases.
