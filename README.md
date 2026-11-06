# omml-converter (Rust)

Convert math equations in Microsoft Word `.docx` documents from **OMML** (Office Math Markup Language, Word's native equation format) into embedded **MathType** OLE objects (`Equation.DSMT4`) with high-fidelity vector previews — without requiring Microsoft Word, MathType, or Windows. A single self-contained binary written in pure Rust.

Double-clicking any converted equation in Microsoft Word or WPS Office opens it directly in **MathType** for full visual editing.

---

## Key Features

- **End-to-End Conversion Pipeline**: Translates `.docx` equations via `OMML → MathML → MTEF 5.0 → Compound File Binary (OLE 2.0) → .docx`.
- **Comprehensive MathType 5 Template & Symbol Support**:
  - **Fences & Intervals**: Parentheses, brackets, braces, open brackets, absolute values, double bars, floor/ceiling, Dirac notation (`⟨ψ|`, `|ψ⟩`, `⟨ϕ|ψ⟩`), and intervals (`(a, b]`, `[a, b)`).
  - **Radicals & Fractions**: Square roots, nth roots, regular fractions, skewed / slash fractions (`a/b`).
  - **Scripts & Limits**: Subscripts, superscripts, sub-superscripts, pre-subscripts/pre-superscripts (`m:sPre`), and limit operators (`\lim`, `\max`, `\min`, `\sup`, `\inf`) with proper size restoration (`FULL` size restoration).
  - **Large Operators & Integrals**:
    - Summations (`∑`), products (`∏`), coproducts (`∐`), unions (`⋃`), intersections (`⋂`).
    - Single, double, and triple integrals ($\int$, $\iint$, $\iiint$).
    - Contour and loop integrals ($\oint$, $\oiint$, $\oiiint$, $\ointclockwise$, $\varointctrclockwise$) mapped to MathType's native composite glyph structures (loop circles in `MT Extra` paired with symbol stems).
  - **Embellishments & Over/Under Bars**: Vectors (`\vec{v}` with expandable right/left arrows), hats (`\hat{x}`), tildes (`\tilde{x}`), arcs (`\overarc{AB}`), single/double overbars (`\overline{AB}`), single/double underbars (`\underline{AB}`), joint status, horizontal/slanted strike-throughs, primes (`′`, `″`, `‴`), and dot accents (`\dot`, `\ddot`, `\dddot`, `\ddddot`).
  - **Extensible Arrows**: Left, right, and bidirectional arrows with labels above and below (`A \xrightarrow{yields} B`, `A \xleftrightarrow[y]{x} B`).
  - **Matrices & Tables**: Multiline equation arrays (`eqArr`), matrices with customizable horizontal/vertical alignment and row/column partition lines.
  - **Long Division**: Elementary long division template with dividend and divisor slots.
  - **Full MathType Typeface & Character Classification**: Accurate classification across `TF_TEXT`, `TF_FUNCTION`, `TF_VARIABLE`, `TF_LCGREEK`, `TF_UCGREEK`, `TF_SYMBOL`, `TF_VECTOR`, `TF_MTEXTRA`, and `TF_EXPAND`.
- **Crisp Vector Previews**: Built-in EMF vector (and optional PNG raster) preview renderer powered by STIX Two Math fonts, ensuring equations render crisply at any zoom level before MathType is even opened.
- **OLE Compatibility**: Generates standard OLE Storage streams (`\x01CompObj`, `\x01Ole`, `\x03ObjInfo`, and `Equation Native` with `EQNOLEFILEHDR`), verified against genuine MathType 5/6 documents.

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

## Usage

```bash
# Basic conversion (writes <input>_mathtype.docx)
omml-converter input.docx

# Specify custom output file
omml-converter input.docx -o output.docx

# Set preview font size (default: 12 pt, matching MathType default)
omml-converter input.docx --font-size 13

# Use PNG raster preview instead of EMF vector
omml-converter input.docx --preview png

# Keep original OMML visible (MathType OLE objects stored in background as fallback)
omml-converter input.docx --omml-fallback

# Verbose progress logging
omml-converter input.docx --verbose

# Quiet mode (suppress progress, print errors only)
omml-converter input.docx --quiet
```

### CLI Options

| Flag / Option | Description |
| --- | --- |
| `<INPUT>` | Path to the source `.docx` file (required). |
| `-o, --output <FILE>` | Destination path (default: `<input>_mathtype.docx`). |
| `--preview <FORMAT>` | Picture format for OLE previews: `emf` (vector, default) or `png` (raster). |
| `--font-size <PT>` | Font size (in points) used to render previews (default: `12`). |
| `--omml-fallback` | Retains OMML visible in Word using `mc:AlternateContent`, storing MathType objects in the background. |
| `-v, --verbose` | Logs per-equation conversion progress and details. |
| `-q, --quiet` | Suppresses info messages; only prints errors. |
| `-h, --help` | Prints help information. |
| `-V, --version` | Prints version. |

---

## Library Usage

Add `omml-converter` to your `Cargo.toml`, or use it directly as an internal library:

```rust
use omml_converter::{convert, convert_bytes, ConversionSummary};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Convert files directly on disk
    let summary: ConversionSummary = convert("document.docx", "document_mathtype.docx")?;
    println!("Converted {}/{} equations.", summary.converted, summary.found);

    // Or convert in-memory bytes (ideal for web services or WASM)
    let docx_bytes = std::fs::read("document.docx")?;
    let (output_bytes, summary) = convert_bytes(&docx_bytes)?;

    Ok(())
}
```

Low-level conversion functions are also exposed:
- `omml::omml_to_mathml`: Decodes OMML XML into an intermediate MathML AST.
- `mtef::encode::mathml_to_mtef_bytes`: Encodes MathML AST into inner MTEF binary records.
- `mtef::make_equation_native`: Packages inner MTEF records with the 28-byte `EQNOLEFILEHDR` header.
- `ole::make_ole`: Packs equation native bytes into an OLE Compound File Binary container.

---

## Architecture

```
.docx (Word Document)
   │
   ▼ [src/docx/package.rs]
Extract word/document.xml & find <m:oMath> / <m:oMathPara>
   │
   ▼ [src/omml/decode.rs]
Decode OMML structure → MathML abstract syntax tree (MathNode)
   │
   ├──▶ [src/render/mod.rs]
   │    Layout engine calculates geometry & renders EMF / PNG preview
   │
   ▼ [src/mtef/encode.rs & src/mtef/templates.rs]
Encode MathML AST → MTEF 5.0 binary records
   │
   ▼ [src/mtef/mod.rs]
Wrap in MTEF header & EQNOLEFILEHDR (Equation Native stream)
   │
   ▼ [src/ole.rs]
Construct OLE 2.0 Compound File Binary (CompObj, Ole, ObjInfo, Equation Native)
   │
   ▼ [src/docx/ole_xml.rs & src/docx/pipeline.rs]
Embed OLE object (.bin) & preview image into .docx package
```

---

## References & Credits

This project builds upon and references the following open-source projects, specifications, and resources:

1. **[mathtypejx](https://github.com/a917470154/mathtypejx)** & **[mathtype_to_mathml](https://github.com/jure/mathtype_to_mathml)**:
   - Provided extensive specifications and fixtures for MTEF v3 and v5 byte streams, template selector mappings, variation bitmasks, and MathType character mapping tables.
2. **[omml-converter (Python)](https://github.com/bohdanbodnarenko/omml-converter)**:
   - Original Python implementation by Bohdan Bodnarenko demonstrating OMML extraction and OLE insertion in `.docx`.
3. **[Design Science MathType MTEF v5 Specification](http://www.dessci.com/en/support/mathtype/tech/mtef5.htm)**:
   - Official technical documentation for the MathType Equation Format (MTEF) v5.
4. **[ECMA-376 (Office Open XML File Formats)](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/)**:
   - Standard reference for Office Math Markup Language (OMML) semantics (§22.1).
5. **[STIX Fonts Project](https://github.com/stipub/stixfonts)**:
   - Bundled fonts in `assets/fonts/` used for generating standalone equation preview pictures, licensed under the [SIL Open Font License 1.1](assets/fonts/OLF.txt).

---

## License

- Code licensed under the [MIT License](LICENSE).
- Bundled STIX fonts licensed under the [SIL Open Font License 1.1](assets/fonts/OLF.txt).
