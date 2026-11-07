# omml-converter (Rust)

A standalone, high-performance **bidirectional math equation converter** for Microsoft Word `.docx` documents between **OMML** (Office Math Markup Language, Word's native equation format) and **MathType** (`Equation.DSMT4` / `Equation.3` OLE objects & WMF/EMF previews) — without requiring Microsoft Word, MathType, or Windows. Written in pure Rust as a single self-contained binary and library.

- **Forward (`OMML → MathType`)**: Converts Word native OMML equations into embedded MathType 5.0 OLE objects (`Equation.DSMT4`) with crisp vector EMF (or PNG) previews. Double-clicking any converted equation in Microsoft Word or WPS Office opens it directly in **MathType** for full visual editing.
- **Reverse (`MathType → OMML`)**: Extracts MTEF v3 (Equation Editor 3.x) and MTEF v5 (MathType 4–7) binary streams from embedded OLE `.bin` containers or WMF (`MTra`) preview images, decodes them into MathML, and replaces the VML `<w:object>` elements with native Word `<m:oMath>` XML.

> **Roadmap Note**: This engine is designed as the core library for an upcoming interactive visual math editor & document conversion suite (planned UI integration).

---

## Key Features

- **Bidirectional End-to-End Conversion Pipelines**:
  - **OMML → MathType**: `.docx → OMML → MathML AST → MTEF 5.0 → OLE 2.0 Compound File Binary + EMF/PNG Preview → .docx`
  - **MathType → OMML**: `.docx (OLE .bin / WMF MTra) → MTEF v3/v5 → MathML AST → OMML (<m:oMath>) → .docx`
  - Processes equations across the entire document package: `word/document.xml`, `word/header*.xml`, `word/footer*.xml`, `word/footnotes.xml`, and `word/endnotes.xml`.
  - **Batch Folder Conversion**: Supports converting an entire directory of `.docx` files recursively while preserving the original subfolder hierarchy and filenames.
- **Comprehensive MathType Template & OMML Element Coverage**:
  - **Fences & Intervals**: Parentheses, brackets, curly braces (including single-sided system braces `{` / `}`), angle brackets, open brackets, absolute values, double vertical bars, floor/ceiling, Dirac notation (`⟨ψ|`, `|ψ⟩`, `⟨ϕ|ψ⟩`), and open/half-open intervals (`(a, b]`, `[a, b)`).
  - **Radicals & Fractions**: Square roots, nth roots, regular bar fractions, skewed / diagonal, and inline slash fractions (`a/b`).
  - **Scripts & Limits**: Subscripts, superscripts, combined sub-superscripts, pre-subscripts/pre-superscripts (`m:sPre` / `tmSUP`), and limit operators (`\lim`, `\max`, `\min`, `\sup`, `\inf`, `\det`, `\gcd`) with under/over bar support (`tvSUBAR`, `tvDUBAR`), `<m:limLoc>` (`undOvr` / `subSup`) preservation, and `FULL` size restoration.
  - **Large Operators & Integrals**:
    - Summations (`∑`), products (`∏`), coproducts (`∐`), unions (`⋃`, `∪`), intersections (`⋂`, `∩`) with `undOvr` or `subSup` limit placement.
    - Single, double, and triple integrals ($\int$, $\iint$, $\iiint$).
    - Contour and loop integrals ($\oint$, $\oiint$, $\oiiint$, $\ointclockwise$, $\varointctrclockwise$) mapped to MathType's native composite glyph structures (loop circles in `MT Extra` paired with symbol stems).
  - **Embellishments, Accents & Over/Under Bars**:
    - Expandable vectors (`\vec`, `\overrightarrow`, `\overleftarrow`, `\overleftrightarrow`, under-arrows) mapped to `tmVEC` and `<m:acc>`.
    - Hats (`\hat`, `\widehat`), tildes (`\tilde`, `\widetilde`), arcs (`\overparen`, `\wideparen`, `\underparen`), single/double overbars (`\overline`), single/double underbars (`\underline`), horizontal braces/brackets (`\overbrace`, `\underbrace`, `\overbracket`, `\underbracket`), joint status, horizontal/slanted strike-throughs, primes (`′`, `″`, `‴`), dot accents (`\dot`, `\ddot`, `\dddot`, `\ddddot`), and Unicode combining mark decomposition.
  - **Extensible Arrows**: Left, right, and bidirectional extensible arrows with upper and lower label slots (`A \xrightarrow{\text{yields}} B`, `A \xleftrightarrow[y]{x} B`).
  - **Matrices, Piecewise Cases & Alignment Arrays**: Multiline equation arrays (`m:eqArr`), piecewise systems, and matrices (`m:m` / `tmMATRIX`) with horizontal/vertical alignment and row/column partition lines.
  - **Boxes & Long Division**: Full and partial border boxes (`top`, `bottom`, `left`, `right`, `actuarial`, strike-throughs via `<m:borderBox>` / `tmBOX`) and elementary long division (`tmLDIV` / `menclose notation="longdiv"`) with optional quotient slot.
  - **Full MathType Typeface & Character Classification**: Accurate bidirectional mapping across `TF_TEXT`, `TF_FUNCTION`, `TF_VARIABLE`, `TF_LCGREEK`, `TF_UCGREEK`, `TF_SYMBOL`, `TF_VECTOR`, `TF_MTEXTRA`, `TF_NUMBER`, and `TF_EXPAND`.
- **Crisp Vector & Glyphs-Assembled Previews**:
  - Built-in EMF vector (and optional PNG raster) preview renderer powered by bundled STIX Two Math fonts.
  - Uses proportional glyph piece assembly for tall delimiters (parentheses, brackets, curly braces), vector arrows, and horizontally scaled wide accents (`\widehat`, `\wideparen`) so equations look crisp at any zoom level before MathType is opened.
- **OLE 2.0 & WMF Compatibility**:
  - Generates standard OLE Storage streams (`\x01CompObj`, `\x01Ole`, `\x03ObjInfo`, and `Equation Native` with `EQNOLEFILEHDR`), verified against MathType 5/6/7.
  - Extracts `Equation Native` / `agn` streams from OLE Compound Files and `MTra` comment records from WMF previews.

---

## Installation

### Prerequisites
Rust **1.75+** (MSRV).

```bash
# Clone the repository
git clone https://github.com/vananh2801/omml-converter-rust.git
cd omml-converter-rust

# Build release binary:
cargo build --release

# The compiled binary will be located at:
# target/release/omml-converter
```

---

## CLI Usage

### 1. Convert Word OMML → MathType (`default`)

```bash
# Basic single-file conversion (writes <input>_mathtype.docx)
omml-converter input.docx

# Specify custom output file or output directory
omml-converter input.docx -o output.docx
omml-converter input.docx -o output_dir/

# Batch convert an entire directory recursively (preserves subfolder structure & filenames)
# Default output directory: <input_dir>_mathtype/
omml-converter ./my_documents
omml-converter ./my_documents -o ./my_documents_mathtype

# Set preview font size (default: 12 pt, matching MathType default)
omml-converter input.docx --font-size 13

# Use PNG raster preview instead of EMF vector
omml-converter input.docx --preview png

# Keep original OMML visible (MathType OLE objects stored in background as fallback)
omml-converter input.docx --omml-fallback
```

### 2. Convert MathType → Word OMML (`-r` / `--to-omml`)

```bash
# Convert a single file (writes <input>_omml.docx)
omml-converter input.docx --to-omml

# Short flag with custom output path
omml-converter input.docx -r -o output_omml.docx

# Batch convert a folder of MathType .docx files to OMML (preserves subfolder structure)
# Default output directory: <input_dir>_omml/
omml-converter ./mathtype_docs -r
omml-converter ./mathtype_docs -r -o ./omml_docs

# Apply a custom math font to all converted OMML runs
omml-converter input.docx -r --math-font "Cambria Math"
```

### CLI Options

| Flag / Option | Description |
| --- | --- |
| `<INPUT>` | Path to the source `.docx` file or directory (scanned recursively for `.docx` files). |
| `-o, --output <PATH>` | Destination `.docx` file or directory path (default: `<input>_mathtype[.docx]` or `<input>_omml[.docx]`). |
| `-r, --to-omml` | Reverse direction: convert embedded MathType equations (`.bin` / `.wmf`) to Word native OMML (`<m:oMath>`). |
| `--math-font <FONT>` | Optional font name applied to converted OMML runs (`<w:rFonts>`) when using `--to-omml`. |
| `--preview <FORMAT>` | Picture format for OLE previews in OMML → MathType mode: `emf` (vector, default) or `png` (raster). |
| `--font-size <PT>` | Font size (in points) used to render previews in OMML → MathType mode (default: `12`). |
| `--omml-fallback` | Retains OMML visible in Word using `mc:AlternateContent`, storing MathType objects in the background. |
| `-v, --verbose` | Logs per-equation conversion progress and details. |
| `-q, --quiet` | Suppresses info messages; only prints errors. |
| `-h, --help` | Prints help information. |
| `-V, --version` | Prints version. |

---

## Library Usage

Add `omml-converter` to your `Cargo.toml`, or use it directly as a Rust library:

```rust
use omml_converter::{
    convert, convert_bytes, convert_to_omml, convert_to_omml_with,
    ConversionSummary, ToOmmlOptions,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Forward: Convert Word OMML -> MathType OLE on disk
    let summary: ConversionSummary = convert("document.docx", "document_mathtype.docx")?;
    println!("OMML -> MathType: converted {}/{} equations.", summary.converted, summary.found);

    // 2. Reverse: Convert MathType OLE/WMF -> Word OMML on disk
    let rev_summary: ConversionSummary = convert_to_omml("document_mathtype.docx", "document_omml.docx")?;
    println!("MathType -> OMML: converted {}/{} equations.", rev_summary.converted, rev_summary.found);

    // 3. Reverse with custom math font option
    let opts = ToOmmlOptions {
        math_font: Some("Cambria Math".to_string()),
    };
    convert_to_omml_with("document_mathtype.docx", "document_cambria.docx", &opts)?;

    // 4. In-memory bytes conversion (ideal for web services, desktop apps, or WASM)
    let docx_bytes = std::fs::read("document.docx")?;
    let (mathtype_docx_bytes, _) = convert_bytes(&docx_bytes)?;

    Ok(())
}
```

### Low-Level Pipeline APIs

Both conversion directions expose their intermediate AST and binary codecs:

- **OMML ↔ MathML**:
  - `omml::omml_to_mathml`: Decodes an `<m:oMath>` / `<m:oMathPara>` XML string into a `MathNode` AST.
  - `omml::mathml_to_omml`: Encodes a `MathNode` AST into an `<m:oMath>` XML string.
  - `omml::mathml_to_omml_xml`: Converts a MathML XML string directly into an `<m:oMath>` XML string.
  - `omml::apply_math_font`: Injects `<w:rPr><w:rFonts .../></w:rPr>` into every `<m:r>` in an OMML fragment.
- **MathML ↔ MTEF ↔ OLE**:
  - `mtef::encode::mathml_to_mtef_bytes`: Encodes a `MathNode` AST into MTEF 5.0 binary records.
  - `mtef::make_equation_native`: Wraps MTEF bytes with the 28-byte `EQNOLEFILEHDR` header.
  - `ole::make_ole`: Packages `Equation Native` bytes into an OLE 2.0 Compound File Binary container.
  - `ole::extract_mtef_from_ole` & `ole::extract_mtef_from_wmf`: Extracts raw MTEF streams from OLE `.bin` containers or WMF (`MTra`) images.
  - `mtef::mtef_to_mathml` & `mtef::ole_to_mathml`: Decodes MTEF v3/v5 binary streams into a `MathNode` AST.

---

## Architecture

```text
                    ┌──────────────────────────────────────────┐
                    │           .docx (Word Document)          │
                    └───────────┬──────────────────▲───────────┘
                                │                  │
        ┌───────────────────────┴──────┐    ┌──────┴───────────────────────┐
        │   Forward: OMML → MathType   │    │   Reverse: MathType → OMML   │
        └───────────────┬──────────────┘    └──────────────▲───────────────┘
                        ▼                                  │
         [src/omml/decode.rs]               [src/docx/pipeline.rs]
         Parse <m:oMath> → MathNode         Replace <w:object> with <m:oMath>
                        │                                  ▲
                        ▼                                  │
         ┌──────────────┴──────────────┐    [src/omml/encode.rs]
         │     MathNode (MathML AST)   ├────Encode MathNode → <m:oMath> XML
         └───┬─────────────────────┬───┘                   ▲
             │                     │                       │
             ▼                     ▼                       │
  [src/render/mod.rs]   [src/mtef/encode.rs]   [src/mtef/decode.rs]
  Layout & Glyph Engine Encode MathNode →      Decode MTEF v3/v5 → MathNode
  (EMF / PNG Preview)   MTEF 5.0 + OLE .bin                ▲
             │                     │                       │
             └──────────┬──────────┘        [src/ole.rs]
                        ▼                   Extract MTEF from OLE .bin / WMF
         [src/docx/ole_xml.rs]
         Embed OLE .bin + Preview in .docx
```

---

## References & Credits

This project is an independent bidirectional conversion engine and library that draws architectural design, format specifications, mathematical typography rules, and reference test cases from the following open-source projects and standards:

1. **[omml-converter](https://github.com/bohdanbodnarenko/omml-converter)** (by Bohdan Bodnarenko):
   - Referenced for the forward (`OMML → MathType`) `.docx` packaging workflow, OLE Compound File Binary container generation, and initial MTEF 5.0 binary generation.
2. **[mathtypejx](https://github.com/a917470154/mathtypejx)** (by a917470154) & **[mathtype_to_mathml](https://github.com/jure/mathtype_to_mathml)** (by Jure Triglav):
   - Referenced for the reverse (`MathType → MathML → OMML`) pipeline: MTEF v3 and v5 binary stream decoding, WMF `MTra` extraction, template selector/variation bitmask tables, sub/superscript base-mover logic, and MathType character set mappings.
3. **[Design Science MathType MTEF v3 & v5 Specifications](http://www.dessci.com/en/support/mathtype/tech/mtef5.htm)**:
   - Technical documentation for the MathType Equation Format (MTEF) v3 and v5 binary records.
4. **[ECMA-376 (Office Open XML File Formats, §22.1 OMML)](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/) / ISO/IEC 29500**:
   - Standard reference for Office Math Markup Language (OMML) XML schema and semantics.
5. **[Microsoft OpenType Specification: The `MATH` Table](https://learn.microsoft.com/en-us/typography/opentype/spec/math)**:
   - Specification for OpenType math constants (`MathConstants`), glyph metrics (`MathGlyphInfo`), and extensible delimiter construction (`MathVariants`).
6. **Donald E. Knuth, *The TeXbook*, Appendix G: "Generating Boxes from Formulas" (Addison-Wesley, 1984)**:
   - Algorithmic foundations for mathematical typesetting: horizontal/vertical spacing rules, fraction positioning (Rule 15), radical placement, and operator alignments (see [TYPESETTING_ROADMAP.md](TYPESETTING_ROADMAP.md)).
7. **[W3C MathML Core Specification](https://w3c.github.io/mathml-core/)**:
   - Standard reference for MathML mathematical layout and token element formatting.
8. **[STIX Fonts Project](https://github.com/stipub/stixfonts)** (by The STI Pub Companies):
   - Embedded fonts in `assets/fonts/` (`STIXTwoText`, `STIXTwoMath`) used for standalone vector (EMF) and raster (PNG) equation preview rendering and OpenType `MATH` table metrics, licensed under the [SIL Open Font License 1.1](assets/fonts/stixfonts-2.13b171/OFL.txt).

Detailed attribution and copyright information are documented in [NOTICE](NOTICE).

---

## License

- Source code is licensed under the [MIT License](LICENSE).
- Bundled STIX fonts (`assets/fonts/stixfonts-2.13b171/`) are licensed under the [SIL Open Font License 1.1](assets/fonts/stixfonts-2.13b171/OFL.txt).
- Comprehensive copyright and third-party notices are available in [NOTICE](NOTICE).
