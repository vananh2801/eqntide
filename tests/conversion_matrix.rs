use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use eqntide::bridge::ConvertSnippetRequest;
use eqntide::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use eqntide::mtef::encode::mathml_to_mtef_bytes;
use eqntide::mtef::{build_mtef, check_mathtype_unsupported, make_equation_native};
use eqntide::ole::{extract_mtef_from_emf, extract_mtef_from_ole, make_ole};
use eqntide::omml::{check_omml_unsupported, mathml_to_omml_xml, omml_to_mathml, NS_M};
use eqntide::render::emf::render_emf_with_mtef;
use eqntide::render::render_preview;
use eqntide::render::svg::render_svg;
use eqntide::service::{convert_snippet, decode_input_to_mathml, parse_latex_to_mathml};
use eqntide::xml;
use eqntide::{
    base64_decode, base64_encode, convert_bytes_to_omml_with, convert_bytes_with, emf_to_omml_xml,
    flat_opc_to_mathml, mathml_to_flat_opc, mathml_to_latex, ole_to_omml_xml,
    upgrade_legacy_mathtype_bytes_with, EmbedMode, FlatOpcMode, MathNode, Options, PreviewFormat,
    ToOmmlOptions, UpgradeOptions, UpgradeTarget,
};
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

struct MatrixCase {
    name: &'static str,
    latex: &'static str,
    expected_tokens: &'static [&'static str],
}

fn collect_tokens(node: &MathNode, out: &mut String) {
    if !node.text.is_empty() {
        out.push_str(&node.text);
        out.push(' ');
    }
    for child in &node.children {
        collect_tokens(child, out);
    }
}

fn assert_has_tokens(node: &MathNode, expected: &[&str], context: &str) {
    let mut flat = String::new();
    collect_tokens(node, &mut flat);
    let compact: String = flat.chars().filter(|c| !c.is_whitespace()).collect();
    for tok in expected {
        assert!(
            flat.contains(tok) || compact.contains(tok),
            "{context}: expected token `{tok}` in `{flat}` (xml: {})",
            node.to_xml()
        );
    }
}

fn matrix_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "fraction_sqrt_quadratic",
            latex: r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
            expected_tokens: &["x", "b", "4", "a", "c", "2"],
        },
        MatrixCase {
            name: "display_and_slash_fractions",
            latex: r"\dfrac{a}{b} + \tfrac{1}{2} + \sfrac{3}{4} + \atopfrac{n}{k}",
            expected_tokens: &["a", "b", "1", "2", "3", "4", "n", "k"],
        },
        MatrixCase {
            name: "nth_root_and_scripts",
            latex: r"\sqrt[3]{x^2 + y_1} + \prescript{235}{92}{U}",
            expected_tokens: &["3", "x", "2", "y", "1", "235", "92", "U"],
        },
        MatrixCase {
            name: "definite_integral_limits",
            latex: r"\int\limits_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}",
            expected_tokens: &["0", "∞", "e", "x", "π", "2"],
        },
        MatrixCase {
            name: "double_triple_contour_integrals",
            latex: r"\iint_{D} f + \iiint_{V} g + \oint_{C} h",
            expected_tokens: &["D", "f", "V", "g", "C", "h"],
        },
        MatrixCase {
            name: "big_operators_sum_prod_union",
            latex: r"\sum_{i=1}^{n} i + \prod_{k=1}^{m} x_k + \bigcup_{j \in J} A_j",
            expected_tokens: &["i", "1", "n", "k", "m", "x", "j", "J", "A"],
        },
        MatrixCase {
            name: "bigop_nolimits_placement",
            latex: r"\bigwedge\nolimits_{a}^{b} P",
            expected_tokens: &["a", "b", "P"],
        },
        MatrixCase {
            name: "fences_dirac_intervals",
            latex: r"\langle \psi | \phi \rangle + \left[ a, b \right) + \left\| \frac{1}{2} x \right\|",
            expected_tokens: &["ψ", "ϕ", "a", "b", "1", "2", "x"],
        },
        MatrixCase {
            name: "fences_floor_ceil_double_bracket",
            latex: r"\left\lfloor \frac{x}{2} \right\rfloor + \left\lceil \frac{y}{3} \right\rceil + \left\llbracket z \right\rrbracket",
            expected_tokens: &["x", "2", "y", "3", "z"],
        },
        MatrixCase {
            name: "matrices_and_cases",
            latex: r"\begin{pmatrix} a & b \\ c & d \end{pmatrix} + \begin{cases} x & x > 0 \\ -x & x \le 0 \end{cases}",
            expected_tokens: &["a", "b", "c", "d", "x", "0"],
        },
        MatrixCase {
            name: "embellishments_dots_primes_harpoons",
            latex: r"\dot{x} + \ddot{y} + \check{a} + \mathring{b} + {u}'' + \overleftharpoon{v}",
            expected_tokens: &["x", "y", "a", "b", "u", "v"],
        },
        MatrixCase {
            name: "bars_braces_underparen",
            latex: r"\overline{AB} + \underline{CD} + \overbrace{x+y}^{n} + \underparen{PQ}",
            expected_tokens: &["A", "B", "C", "D", "x", "y", "n", "P", "Q"],
        },
        MatrixCase {
            name: "labeled_extensible_arrows",
            latex: r"A \overset{f}{\underset{g}{\rightarrow}} B + C \overset{k_1}{\underset{k_2}{\leftrightharpoons}} D",
            expected_tokens: &["A", "f", "g", "B", "C", "k", "1", "2", "D"],
        },
        MatrixCase {
            name: "greek_styled_and_symbols",
            latex: r"\alpha + \beta + \Gamma + \Omega + \mathbb{R} + \mathcal{L} + \forall x \in \mathbb{N}",
            expected_tokens: &["α", "β", "Γ", "Ω", "R", "∀", "∈", "x", "N"],
        },
        MatrixCase {
            name: "mathtype_only_longdiv_and_roundedbox",
            latex: r"\enclose{longdiv}{144} + \enclose{roundedbox}{x+y}",
            expected_tokens: &["144", "x", "y"],
        },
    ]
}

#[test]
fn matrix_full_format_roundtrip_across_palette_categories() {
    for case in matrix_cases() {
        let math = parse_latex_to_mathml(case.latex)
            .unwrap_or_else(|e| panic!("[{}] parse_latex_to_mathml failed: {e}", case.name));
        assert_has_tokens(&math, case.expected_tokens, case.name);

        // 1. MathML XML roundtrip
        let mml_xml = math.to_xml();
        let mml_reparsed = MathNode::parse(&mml_xml)
            .unwrap_or_else(|e| panic!("[{}] MathNode::parse failed: {e}", case.name));
        assert_has_tokens(
            &mml_reparsed,
            case.expected_tokens,
            &format!("{}:mathml", case.name),
        );

        // 2. LaTeX output + re-parse
        let exported_latex = mathml_to_latex(&math);
        assert!(
            !exported_latex.trim().is_empty(),
            "[{}] exported latex is empty",
            case.name
        );
        let latex_reparsed = parse_latex_to_mathml(&exported_latex).unwrap_or_else(|e| {
            panic!(
                "[{}] re-parsing exported latex `{exported_latex}` failed: {e}",
                case.name
            )
        });
        assert_has_tokens(
            &latex_reparsed,
            case.expected_tokens,
            &format!("{}:latex", case.name),
        );

        // 3. OMML <-> MathML <-> Flat OPC (when supported by OMML)
        if check_omml_unsupported(&math).is_none() {
            let omml_xml = mathml_to_omml_xml(&math)
                .unwrap_or_else(|e| panic!("[{}] mathml_to_omml_xml failed: {e}", case.name));
            let omml_el = xml::parse(omml_xml.as_bytes()).expect("valid OMML XML");
            assert!(omml_el.local == "oMath" || omml_el.local == "oMathPara");
            let from_omml = omml_to_mathml(&omml_el)
                .unwrap_or_else(|e| panic!("[{}] omml_to_mathml failed: {e}", case.name));
            assert_has_tokens(
                &from_omml,
                case.expected_tokens,
                &format!("{}:omml", case.name),
            );

            let flat_omml = mathml_to_flat_opc(&math, FlatOpcMode::ModernOmml, 12.0)
                .unwrap_or_else(|e| panic!("[{}] flat_opc OMML failed: {e}", case.name));
            assert!(flat_omml.contains("<pkg:package"));
            assert!(flat_omml.contains("oMath"));
            let from_flat_omml = flat_opc_to_mathml(&flat_omml)
                .unwrap_or_else(|e| panic!("[{}] flat_opc_to_mathml OMML failed: {e}", case.name));
            assert_has_tokens(
                &from_flat_omml,
                case.expected_tokens,
                &format!("{}:flat_opc_omml", case.name),
            );
        }

        // 4. MathType MTEF <-> OLE <-> EMF <-> Flat OPC <-> OMML (when supported by MathType)
        if check_mathtype_unsupported(&math).is_none() {
            let records = mathml_to_mtef_bytes(&math)
                .unwrap_or_else(|e| panic!("[{}] mathml_to_mtef_bytes failed: {e}", case.name));
            assert!(!records.is_empty());
            let full_mtef = build_mtef(&records);
            assert_eq!(full_mtef[0], 5, "[{}] expected MTEF v5", case.name);

            let from_mtef = mtef_to_mathml(&full_mtef)
                .unwrap_or_else(|| panic!("[{}] mtef_to_mathml failed", case.name));
            assert_has_tokens(
                &from_mtef,
                case.expected_tokens,
                &format!("{}:mtef", case.name),
            );

            // OLE compound binary roundtrip
            let native_stream = make_equation_native(&records);
            let ole_bytes = make_ole(&native_stream)
                .unwrap_or_else(|e| panic!("[{}] make_ole failed: {e}", case.name));
            assert_eq!(
                &ole_bytes[0..8],
                &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
            );
            assert_eq!(ole_bytes.len() % 512, 0);

            let extracted_mtef = extract_mtef_from_ole(&ole_bytes)
                .unwrap_or_else(|| panic!("[{}] extract_mtef_from_ole failed", case.name));
            assert_eq!(extracted_mtef, full_mtef);

            let from_ole = ole_to_mathml(&ole_bytes)
                .unwrap_or_else(|| panic!("[{}] ole_to_mathml failed", case.name));
            assert_has_tokens(
                &from_ole,
                case.expected_tokens,
                &format!("{}:ole", case.name),
            );

            // EMF with embedded MTEF roundtrip
            let emf = render_emf_with_mtef(&math, 12.0, Some(&full_mtef))
                .unwrap_or_else(|e| panic!("[{}] render_emf_with_mtef failed: {e}", case.name));
            assert!(emf.width_pt > 0.0 && emf.height_pt > 0.0);
            assert_eq!(&emf.emf[40..44], b" EMF");
            let emf_mtef = extract_mtef_from_emf(&emf.emf)
                .unwrap_or_else(|| panic!("[{}] extract_mtef_from_emf failed", case.name));
            assert_eq!(emf_mtef, full_mtef);

            // Cross OLE/EMF -> OMML helpers when OMML also supports the formula
            if check_omml_unsupported(&math).is_none() {
                let omml_from_ole = ole_to_omml_xml(&ole_bytes)
                    .unwrap_or_else(|e| panic!("[{}] ole_to_omml_xml failed: {e}", case.name));
                assert!(omml_from_ole.contains("oMath"));
                let omml_from_emf = emf_to_omml_xml(&emf.emf)
                    .unwrap_or_else(|e| panic!("[{}] emf_to_omml_xml failed: {e}", case.name));
                assert!(omml_from_emf.contains("oMath"));
            }

            // Flat OPC MathType OLE+EMF roundtrip
            let flat_mt = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0)
                .unwrap_or_else(|e| panic!("[{}] flat_opc MathType failed: {e}", case.name));
            assert!(flat_mt.contains("Equation.DSMT4"));
            assert!(flat_mt.contains("pkg:binaryData"));
            let from_flat_mt = flat_opc_to_mathml(&flat_mt).unwrap_or_else(|e| {
                panic!("[{}] flat_opc_to_mathml MathType failed: {e}", case.name)
            });
            assert_has_tokens(
                &from_flat_mt,
                case.expected_tokens,
                &format!("{}:flat_opc_mathtype", case.name),
            );
        }

        // 5. Visual raster (PNG) and vector (SVG) outputs
        let png = render_preview(&math, 12.0)
            .unwrap_or_else(|e| panic!("[{}] render_preview PNG failed: {e}", case.name));
        assert!(png.png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(png.width_pt > 0.0 && png.height_pt > 0.0);

        let svg = render_svg(&math, 12.0)
            .unwrap_or_else(|e| panic!("[{}] render_svg failed: {e}", case.name));
        assert!(svg.svg.contains("<svg") && svg.svg.contains("</svg>"));
        assert!(svg.width_pt > 0.0 && svg.height_pt > 0.0);
    }
}

#[test]
fn matrix_legacy_fixtures_ole_and_emf_convert_cleanly() {
    let fixture_dir = Path::new("tests/fixtures/legacy_preview");
    for (stem, expected_tokens) in [
        ("angle", &["O", "u", "v"][..]),
        ("fraction", &["π", "3"][..]),
        ("system", &["u", "n"][..]),
    ] {
        let ole_bytes =
            fs::read(fixture_dir.join(format!("{stem}.ole"))).expect("read ole fixture");
        let emf_bytes =
            fs::read(fixture_dir.join(format!("{stem}.emf"))).expect("read emf fixture");

        let from_ole =
            ole_to_mathml(&ole_bytes).unwrap_or_else(|| panic!("ole_to_mathml failed for {stem}"));
        assert_has_tokens(&from_ole, expected_tokens, &format!("fixture:{stem}:ole"));

        let omml_from_ole = ole_to_omml_xml(&ole_bytes)
            .unwrap_or_else(|e| panic!("ole_to_omml_xml failed for {stem}: {e}"));
        assert!(omml_from_ole.contains("oMath"));

        let omml_from_emf = emf_to_omml_xml(&emf_bytes)
            .unwrap_or_else(|e| panic!("emf_to_omml_xml failed for {stem}: {e}"));
        assert!(omml_from_emf.contains("oMath"));

        let mtef_ole = extract_mtef_from_ole(&ole_bytes).expect("extract mtef from ole");
        let mtef_emf = extract_mtef_from_emf(&emf_bytes).expect("extract mtef from emf");
        assert!(
            mtef_ole
                .windows(mtef_emf.len())
                .any(|w| w == mtef_emf.as_slice()),
            "{stem}: legacy EMF bare MTEF records must match paired OLE stream"
        );
        let decoded = mtef_to_mathml(&mtef_ole).expect("decode mtef");
        let latex = mathml_to_latex(&decoded);
        assert!(!latex.is_empty());

        let png = render_preview(&decoded, 12.0).expect("render png from legacy fixture");
        assert!(png.png.starts_with(b"\x89PNG\r\n\x1a\n"));
        let svg = render_svg(&decoded, 12.0).expect("render svg from legacy fixture");
        assert!(svg.svg.contains("<svg"));
    }
}

fn build_synthetic_omml_docx(equations_latex: &[&str]) -> Vec<u8> {
    let mut paragraphs = String::new();
    for (idx, latex) in equations_latex.iter().enumerate() {
        let math = parse_latex_to_mathml(latex).expect("parse latex for docx");
        let omml = mathml_to_omml_xml(&math).expect("encode omml for docx");
        paragraphs.push_str(&format!(
            "<w:p><w:r><w:t xml:space=\"preserve\">Eq {idx}: </w:t></w:r>{omml}<w:r><w:t>.</w:t></w:r></w:p>"
        ));
    }

    let doc_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>{paragraphs}<w:sectPr/></w:body>
</w:document>"#
    );
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;
    let doc_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#;

    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let opts = FileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();
        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(rels.as_bytes()).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(doc_xml.as_bytes()).unwrap();
        zip.start_file("word/_rels/document.xml.rels", opts)
            .unwrap();
        zip.write_all(doc_rels.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf.into_inner()
}

fn read_zip_entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("open zip");
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        out.push((file.name().to_string(), data));
    }
    out
}

#[test]
fn matrix_docx_end_to_end_forward_reverse_and_upgrade() {
    let formulas = [
        r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        r"\int_{0}^{1} x^2 dx",
        r"\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}",
        r"\prescript{14}{6}{C}",
        r"\alpha + \beta = \gamma",
    ];
    let src_docx = build_synthetic_omml_docx(&formulas);

    // 1. OMML -> MathType (EMF preview)
    let (mt_emf_docx, summary_emf) = convert_bytes_with(
        &src_docx,
        &Options {
            mode: EmbedMode::Preview,
            font_pt: 12.0,
            preview: PreviewFormat::Emf,
        },
    )
    .expect("convert_bytes_with EMF");
    assert_eq!(summary_emf.found, formulas.len());
    assert_eq!(summary_emf.converted, formulas.len());
    assert_eq!(summary_emf.errors, 0);

    let entries = read_zip_entries(&mt_emf_docx);
    let doc_xml = entries
        .iter()
        .find(|(n, _)| n == "word/document.xml")
        .map(|(_, d)| String::from_utf8(d.clone()).unwrap())
        .unwrap();
    let doc_el = xml::parse(doc_xml.as_bytes()).unwrap();
    assert_eq!(
        doc_el
            .iter()
            .into_iter()
            .filter(|e| e.is(NS_M, "oMath"))
            .count(),
        0,
        "Preview mode must replace all oMath elements"
    );
    assert!(doc_xml.contains("Equation.DSMT4"));

    let bins: Vec<_> = entries
        .iter()
        .filter(|(n, _)| n.starts_with("word/embeddings/") && n.ends_with(".bin"))
        .collect();
    let emfs: Vec<_> = entries
        .iter()
        .filter(|(n, _)| n.starts_with("word/media/") && n.ends_with(".emf"))
        .collect();
    assert_eq!(bins.len(), formulas.len());
    assert_eq!(emfs.len(), formulas.len());
    for ((bin_name, bin), (emf_name, emf)) in bins.iter().zip(emfs.iter()) {
        let ole_mtef =
            extract_mtef_from_ole(bin).unwrap_or_else(|| panic!("missing MTEF in {bin_name}"));
        let emf_mtef =
            extract_mtef_from_emf(emf).unwrap_or_else(|| panic!("missing MTEF in {emf_name}"));
        assert_eq!(ole_mtef, emf_mtef, "OLE/EMF MTEF mismatch");
    }

    // 2. OMML -> MathType (PNG preview)
    let (mt_png_docx, summary_png) = convert_bytes_with(
        &src_docx,
        &Options {
            mode: EmbedMode::Preview,
            font_pt: 12.0,
            preview: PreviewFormat::Png,
        },
    )
    .expect("convert_bytes_with PNG");
    assert_eq!(summary_png.converted, formulas.len());
    let png_entries = read_zip_entries(&mt_png_docx);
    let pngs: Vec<_> = png_entries
        .iter()
        .filter(|(n, _)| n.starts_with("word/media/") && n.ends_with(".png"))
        .collect();
    assert_eq!(pngs.len(), formulas.len());
    for (_, data) in pngs {
        assert!(data.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    // 3. OMML -> MathType (OmmlFallback via mc:AlternateContent)
    let (fallback_docx, summary_fb) = convert_bytes_with(
        &src_docx,
        &Options {
            mode: EmbedMode::OmmlFallback,
            font_pt: 12.0,
            preview: PreviewFormat::Emf,
        },
    )
    .expect("convert_bytes_with OmmlFallback");
    assert_eq!(summary_fb.converted, formulas.len());
    let fb_doc = read_zip_entries(&fallback_docx)
        .into_iter()
        .find(|(n, _)| n == "word/document.xml")
        .map(|(_, d)| String::from_utf8(d).unwrap())
        .unwrap();
    assert!(fb_doc.contains("mc:AlternateContent"));
    assert!(fb_doc.contains("Equation.DSMT4"));
    assert!(fb_doc.contains("oMath"));

    // 4. Reverse MathType -> OMML with math_font
    let (omml_back_docx, rev_summary) = convert_bytes_to_omml_with(
        &mt_emf_docx,
        &ToOmmlOptions {
            math_font: Some("Cambria Math".into()),
        },
    )
    .expect("convert_bytes_to_omml_with");
    assert_eq!(rev_summary.found, formulas.len());
    assert_eq!(rev_summary.converted, formulas.len());
    let rev_entries = read_zip_entries(&omml_back_docx);
    let rev_doc = rev_entries
        .iter()
        .find(|(n, _)| n == "word/document.xml")
        .map(|(_, d)| String::from_utf8(d.clone()).unwrap())
        .unwrap();
    let rev_el = xml::parse(rev_doc.as_bytes()).unwrap();
    assert_eq!(
        rev_el
            .iter()
            .into_iter()
            .filter(|e| e.is(NS_M, "oMath"))
            .count(),
        formulas.len()
    );
    assert!(rev_doc.contains("Cambria Math"));
    assert!(
        !rev_entries
            .iter()
            .any(|(n, _)| n.starts_with("word/embeddings/") || n.starts_with("word/media/")),
        "orphaned OLE/media files must be cleaned up"
    );

    // 5. Upgrade MathType -> ModernOmml & ModernMathType
    let (up_omml_docx, up_omml_sum) = upgrade_legacy_mathtype_bytes_with(
        &mt_emf_docx,
        &UpgradeOptions {
            target: UpgradeTarget::ModernOmml,
            font_pt: 12.0,
            math_font: Some("STIX Two Math".into()),
        },
    )
    .expect("upgrade to ModernOmml");
    assert_eq!(up_omml_sum.found, formulas.len());
    assert_eq!(up_omml_sum.converted, formulas.len());
    let up_omml_doc = read_zip_entries(&up_omml_docx)
        .into_iter()
        .find(|(n, _)| n == "word/document.xml")
        .map(|(_, d)| String::from_utf8(d).unwrap())
        .unwrap();
    assert!(up_omml_doc.contains("STIX Two Math"));

    let (up_mt_docx, up_mt_sum) = upgrade_legacy_mathtype_bytes_with(
        &mt_emf_docx,
        &UpgradeOptions {
            target: UpgradeTarget::ModernMathType,
            font_pt: 14.0,
            math_font: None,
        },
    )
    .expect("upgrade to ModernMathType");
    assert_eq!(up_mt_sum.found, formulas.len());
    assert_eq!(up_mt_sum.converted, formulas.len());
    let up_mt_entries = read_zip_entries(&up_mt_docx);
    assert_eq!(
        up_mt_entries
            .iter()
            .filter(|(n, _)| n.starts_with("word/embeddings/") && n.ends_with(".bin"))
            .count(),
        formulas.len()
    );
}

#[test]
fn matrix_service_snippet_conversions_and_422_guards() {
    let latex = r"\frac{x+1}{2} + \int_{0}^{\pi} \sin t\,dt";
    let math = parse_latex_to_mathml(latex).unwrap();
    let mathml_xml = math.to_xml();
    let omml_xml = mathml_to_omml_xml(&math).unwrap();
    let flat_omml = mathml_to_flat_opc(&math, FlatOpcMode::ModernOmml, 12.0).unwrap();
    let mtef_records = mathml_to_mtef_bytes(&math).unwrap();
    let full_mtef = build_mtef(&mtef_records);
    let ole_b64 = base64_encode(&make_ole(&make_equation_native(&mtef_records)).unwrap());
    let emf_b64 = base64_encode(
        &render_emf_with_mtef(&math, 12.0, Some(&full_mtef))
            .unwrap()
            .emf,
    );

    let inputs = [
        ("latex", latex),
        ("mathml", mathml_xml.as_str()),
        ("omml", omml_xml.as_str()),
        ("flat_opc", flat_omml.as_str()),
        ("ole_base64", ole_b64.as_str()),
        ("emf_base64", emf_b64.as_str()),
    ];

    for (from_fmt, data) in inputs {
        let decoded = decode_input_to_mathml(data, from_fmt)
            .unwrap_or_else(|e| panic!("decode_input_to_mathml({from_fmt}) failed: {e}"));
        assert_has_tokens(&decoded, &["x", "1", "2", "0", "π", "t"], from_fmt);

        for to_fmt in [
            "flat_opc_omml",
            "flat_opc_mathtype",
            "omml",
            "mathml",
            "latex",
            "emf_base64",
            "png_base64",
        ] {
            let res = convert_snippet(&ConvertSnippetRequest {
                data: data.into(),
                from: from_fmt.into(),
                to: to_fmt.into(),
                font_pt: 12.0,
            })
            .unwrap_or_else(|e| panic!("convert_snippet({from_fmt} -> {to_fmt}) failed: {e}"));

            if let Some(path) = res.emf_path {
                assert!(Path::new(&path).exists(), "missing temp emf_path {path}");
                let _ = fs::remove_file(path);
            }

            match to_fmt {
                "flat_opc_omml" => assert!(res.result.contains("oMath")),
                "flat_opc_mathtype" => {
                    assert!(res.result.contains("Equation.DSMT4"));
                    assert!(res.width_pt.unwrap_or(0.0) > 0.0);
                    assert!(res.height_pt.unwrap_or(0.0) > 0.0);
                }
                "omml" => assert!(res.result.contains("oMath")),
                "mathml" => assert!(res.result.contains("<math")),
                "latex" => assert!(!res.result.trim().is_empty()),
                "emf_base64" => {
                    let bytes = base64_decode(&res.result).unwrap();
                    assert_eq!(&bytes[40..44], b" EMF");
                }
                "png_base64" => {
                    let bytes = base64_decode(&res.result).unwrap();
                    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
                }
                _ => unreachable!(),
            }
        }
    }

    // Unknown target returns 400
    let err = convert_snippet(&ConvertSnippetRequest {
        data: "x+1".into(),
        from: "latex".into(),
        to: "unknown_target".into(),
        font_pt: 12.0,
    })
    .unwrap_err();
    assert_eq!(err.status, 400);

    // Unsupported OMML construct returns 422
    let err_422 = convert_snippet(&ConvertSnippetRequest {
        data: r"\enclose{longdiv}{144}".into(),
        from: "latex".into(),
        to: "omml".into(),
        font_pt: 12.0,
    })
    .unwrap_err();
    assert_eq!(err_422.status, 422);
}
