# omml-converter (Rust)

Convert the math equations in a Microsoft Word `.docx` from **OMML** (Office Math
Markup Language, Word's native equation format) into embedded **MathType** OLE
objects (`Equation.DSMT4`) — without Word, MathType, or any Microsoft software
installed. A single static binary; no Python, no runtime.

This is a Rust fork of the Python
[`omml-converter`](https://github.com/bohdanbodnarenko/omml-converter). For the
same input it produces **byte-identical** `.bin` OLE objects and an equivalent
`document.xml`.

The original OMML is preserved as a fallback, so every equation stays visible in
ordinary Word while MathType-aware tooling can pick up the embedded objects.

## Installation

```bash
cargo install --path .          # from a checkout
# or build a release binary:
cargo build --release           # → target/release/omml-converter
```

Requires Rust **1.75+**. A `Cargo.lock` is committed and pins dependency
versions that build on 1.75.

## Usage

```bash
omml-converter input.docx                     # writes input_mathtype.docx
omml-converter input.docx -o converted.docx
omml-converter input.docx --font-size 13      # size the equation pictures are drawn at
omml-converter input.docx --omml-fallback     # keep the OMML visible (see below)
omml-converter input.docx --verbose           # show each equation as it converts
omml-converter input.docx --quiet             # only print errors
omml-converter --version
```

| Option | Description |
| --- | --- |
| `input` | Input `.docx` file (required). |
| `-o`, `--output` | Output path (default: `<input>_mathtype.docx`). |
| `--font-size PT` | Size the preview pictures are drawn at, 4–72 (default `12`, MathType's default). |
| `--omml-fallback` | Keep the original OMML visible; MathType objects are stored but not shown. |
| `-v`, `--verbose` | Log every equation as it is converted. |
| `-q`, `--quiet` | Suppress progress; print errors only. |
| `--preview png` | Use .png as OLE preview. |

| Exit code | Meaning |
| --- | --- |
| `0` | Success — all equations converted (or none present). |
| `1` | Bad input (missing / non-`.docx` / unreadable) or one or more equations failed. |
| `2` | Invalid command-line arguments. |

The output `.docx` is always written when the input is readable; equations that
fail to convert are left as their original OMML.

### What you get

**Default mode.** Every equation becomes what Word itself writes for a MathType
equation: an inline `w:object` holding an `Equation.DSMT4` OLE object **and a
preview picture** (`v:imagedata`). Word, WPS and LibreOffice display the picture;
MathType (on Windows/macOS) opens the object for editing. The original OMML is
replaced. Pictures are drawn by the built-in renderer (STIX Two fonts layout) 
at 288 dpi, so a 4 000-equation document grows by roughly 10 MB.

A bare OLE object with *no* picture shows as an empty frame in every viewer —
which is why earlier versions of this tool (and the Python original) appeared to
"lose" the equations.

**`--omml-fallback`.** The OLE object is wrapped in `mc:AlternateContent` whose
`Requires` names a namespace no application knows, so viewers always pick the
OMML branch. Equations stay visible as normal Word equations; the MathType
objects are only stored for tools that read the XML directly.

## Use as a library

```rust
let summary = omml_converter::convert("input.docx", "output.docx")?;
println!("{}/{} converted, {} skipped, {} failed",
         summary.converted, summary.found, summary.skipped, summary.errors);
```

`convert_bytes(&[u8]) -> Result<(Vec<u8>, ConversionSummary)>` works fully in
memory (handy for servers or WASM-style use). The individual stages are public
too: `omml_to_mathml`, `mathml_to_mtef_bytes`, `make_equation_native`, `make_ole`.

## How it works

```
.docx ──▶ OMML  ──▶  MathML  ──▶  MTEF 5.0  ──▶  OLE (CFB) ──▶ .docx
        omml::    mtef::         ole.rs       (re-zipped by docx::)
        decode    encode
                     └──▶ render:: ──▶ preview EMF (or PNG)
```

MathML is the hub: each format has a `decode` (format → MathML) and an
`encode` (MathML → format) module, so the planned MTEF → OMML direction is
`mtef::decode` + `omml::encode`.

```
src/
├── lib.rs, main.rs, cli.rs, error.rs, xml.rs, mathml.rs, ole.rs
├── symbols/          shared symbol tables, one file per MathType palette
│   └── relations, operators, arrows, logic, sets, misc, greek,
│       embellishments, fences, functions
├── omml/             decode.rs (OMML → MathML); encode.rs planned
├── mtef/             records, typeface, templates, encode.rs; decode.rs planned
├── docx/             options, consts, ole_xml, package, job, pipeline
└── render/           mod.rs (EMF/PNG preview), font.rs, canvas.rs, png.rs, emf.rs
    └── layout/       one module per MathType palette group
        └── row, fence, fraction, radical, script, bigop,
            integral, bar, embellish, matrix, geom, boxes
```

## Limitations

* Same as upstream: matrices are flattened in the MathType data; phantoms are
  dropped; pre-scripts (`m:sPre`) are not supported.
* The preview renderer is a compact layout engine, not MathType's:
  spacing and glyph shapes differ slightly. MathType redraws the picture when you
  edit the object.
* **Not verified against MathType itself** (it needs Windows/macOS). The
  container and streams were checked against a genuine MathType file and with
  `olefile`; the pictures with LibreOffice.
* Vertical alignment relies on `w:position`, which Word and WPS honour but
  LibreOffice ignores for objects (they sit slightly high there).

## Development

```bash
cargo run --example render_docx -- in.docx out_dir   # dump equation pictures as EMFs/PNGs
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

## License

[MIT](LICENSE) © Bohdan Bodnarenko (original Python implementation).

The bundled fonts in `assets/fonts/` (STIX Fonts) are
licensed under the SIL Open Font License 1.1 — see `assets/fonts/OLF.txt` and `assets/fonts/OLF-FAQ.txt`.
