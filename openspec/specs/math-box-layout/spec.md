# math-box-layout Specification

## Purpose

Defines TeX Appendix G and OpenType `MATH` box layout behavior in the shared equation renderer while preserving MathType/OMML parity and visual compatibility with the bundled `STIXTwoText` and `STIXTwoMath` fonts.

## Requirements

### Requirement: Unified math axis height from font constants
All layout modules (`bigop`, `delimiter`, `enclosure`, `fence`, `integral`, `matrix`, `row`, `script`) SHALL obtain the mathematical axis height from `self.font_set.constants().axis_height` rather than a hardcoded constant.

#### Scenario: Axis-aligned elements share the font table axis height
- **WHEN** big operators, stretchy delimiters, evaluation bars, matrices, or horizontal strikes are laid out
- **THEN** vertical centering around the mathematical axis uses `constants.axis_height * em` and produces identical coordinates for `STIXTwoMath-Regular.ttf`

### Requirement: Style-aware inter-atom horizontal spacing
`Layout::row` and `Layout::spacing` SHALL apply *The TeXbook* (Ch. 18 / Appendix G Rule 20) inter-atom spacing conditional on `MathContext`: full `THIN` (`3/18 em`), `MED` (`4/18 em`), and `THICK` (`5/18 em`) spacing in `Display` and `Text` styles, while suppressing `MED` (`Bin`) and `THICK` (`Rel`) spacing (and non-operator `THIN` spacing) in `Script` and `ScriptScript` styles.

#### Scenario: Display and Text rows preserve standard TeX operator spacing
- **WHEN** binary operators, relations, large operators, and punctuation are laid out in `MathContext::DISPLAY` or `MathContext::TEXT`
- **THEN** adjacent atom classes receive the existing `3/18 em`, `4/18 em`, and `5/18 em` spacing

#### Scenario: Script and ScriptScript rows compact binary and relational spacing
- **WHEN** expressions such as `x^{a+b}`, `x_{i=1}`, or `A_{i,j}` are laid out inside `Script` or `ScriptScript` contexts
- **THEN** binary (`Bin`) and relational (`Rel`) operators and punctuation (`Punct`) omit `MED`/`THICK`/`THIN` inter-atom padding while retaining `THIN` spacing adjacent to `Op` atoms where TeX specifies it

### Requirement: Selective preservation of MathType/OMML parity and font-fitted geometry
Layout standardization SHALL preserve MathType/OMML full-size fraction numerators/denominators in `Display`/`Text` styles, continuous delimiter assembly, and existing stretchy arrow and contour integral assembly, and every layout adjustment MUST be visually verified against `STIXTwoText` + `STIXTwoMath` rendered sheets.

#### Scenario: Visual inspection sheets remain clean and round-trip consistent
- **WHEN** `cargo test --test export_verification -- --nocapture` renders `target/test_exports/visual/`
- **THEN** `14_roundtrip_p1.png`, `14_roundtrip_p2.png`, and `15_layout_edge_cases.png` show balanced fences, legible non-colliding sub/superscripts, and consistent `[Orig]`, `[via OMML]`, and `[via OLE/MTEF]` structure
