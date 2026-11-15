use std::fs;
use std::path::Path;

use eqntide::catalog::{preview_template, template, ENTRIES, GROUPS};
use eqntide::mtef::decode::mtef_to_mathml;
use eqntide::mtef::encode::mathml_to_mtef_bytes;
use eqntide::mtef::{build_mtef, make_equation_native};
use eqntide::ole::{extract_mtef_from_emf, extract_mtef_from_ole, make_ole, read_ole_stream};
use eqntide::omml::{mathml_to_omml_xml, omml_to_mathml};
use eqntide::render::emf::render_emf_with_mtef;
use eqntide::render::render_preview;
use eqntide::service::parse_latex_to_mathml;
use eqntide::xml;
use eqntide::MathNode;

struct TestCase {
    slug: &'static str,
    title: &'static str,
    latex: &'static str,
}

#[test]
fn test_export_all_formats_and_verify_standards() {
    let out_dir = Path::new("target/test_exports");
    fs::create_dir_all(out_dir).expect("create test_exports dir");

    let test_cases = [
        TestCase {
            slug: "01_quadratic",
            title: "Fraction & Square Root (Quadratic Formula)",
            latex: r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        },
        TestCase {
            slug: "02_integral_limits",
            title: "Integral with Upper/Lower Limits (Definite Integral)",
            latex: r"\int\limits_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}",
        },
        TestCase {
            slug: "03_dirac_interval",
            title: "Dirac Bra-Ket & Half-Open Interval",
            latex: r"\langle \psi | \phi \rangle + \left[ a, b \right)",
        },
        TestCase {
            slug: "04_matrix_3x3",
            title: "3x3 Matrix with Parentheses",
            latex: r"\begin{pmatrix} 1 & 0 & 0 \\ 0 & 1 & 0 \\ 0 & 0 & 1 \end{pmatrix}",
        },
        TestCase {
            slug: "05_labeled_arrow_box",
            title: "Labeled Arrow & Boxed Cancel",
            latex: r"A \overset{f}{\rightarrow} B + \boxed{x} + \cancel{y}",
        },
        TestCase {
            slug: "06_double_bracket_bar",
            title: "Double Brackets & Double Overbar",
            latex: r"\left\llbracket \frac{x}{y} \right\rrbracket + \doubleoverline{z}",
        },
    ];

    println!("\n=== STARTING FORMAT EXPORT VERIFICATION (MathML, OMML, OLE/MTEF, PNG, EMF) ===");

    for tc in &test_cases {
        println!("\n------------------------------------------------------------");
        println!("Sample: {} ({})", tc.title, tc.slug);
        println!("LaTeX: {}", tc.latex);

        // 1. Convert LaTeX -> MathML
        let math = parse_latex_to_mathml(tc.latex)
            .unwrap_or_else(|e| panic!("LaTeX parse failed for {}: {e}", tc.slug));
        let mathml_xml = math.to_xml();

        // 2. Export MathML XML
        let mathml_path = out_dir.join(format!("{}.mathml", tc.slug));
        fs::write(&mathml_path, mathml_xml.as_bytes()).expect("write mathml file");
        assert!(mathml_path.exists());
        let read_mathml = fs::read_to_string(&mathml_path).expect("read back mathml");
        let parsed_xml = xml::parse(read_mathml.as_bytes()).expect("valid MathML XML");
        assert_eq!(parsed_xml.local, "math");
        println!("  [OK] MathML: {} bytes, root=<math>", read_mathml.len());

        // 3. Export Word OMML XML
        let omml_xml = mathml_to_omml_xml(&math)
            .unwrap_or_else(|e| panic!("OMML generation failed for {}: {e}", tc.slug));
        let omml_path = out_dir.join(format!("{}.omml.xml", tc.slug));
        fs::write(&omml_path, omml_xml.as_bytes()).expect("write omml file");
        assert!(omml_path.exists());
        let read_omml = fs::read_to_string(&omml_path).expect("read back omml");
        let parsed_omml_el = xml::parse(read_omml.as_bytes()).expect("valid OMML XML");
        assert!(
            parsed_omml_el.local == "oMath" || parsed_omml_el.local == "oMathPara",
            "OMML local name must be oMath or oMathPara, actual: {}",
            parsed_omml_el.local
        );
        // Round-trip OMML -> MathML
        let back_from_omml = omml_to_mathml(&parsed_omml_el).expect("decode OMML to MathML");
        assert!(
            !back_from_omml.children.is_empty(),
            "OMML decode must not be empty"
        );
        println!(
            "  [OK] OMML: {} bytes, valid m:oMath namespace, OMML -> MathML round-trip succeeded",
            read_omml.len()
        );

        // 4. Export OLE binary containing MathType Equation Native Stream (MTEF)
        let mtef_bytes = mathml_to_mtef_bytes(&math)
            .unwrap_or_else(|e| panic!("MTEF encoding failed for {}: {e}", tc.slug));
        assert!(!mtef_bytes.is_empty(), "MTEF bytes must not be empty");
        let native_stream = make_equation_native(&mtef_bytes);
        let ole_bytes = make_ole(&native_stream)
            .unwrap_or_else(|e| panic!("OLE compound creation failed for {}: {e}", tc.slug));
        let ole_path = out_dir.join(format!("{}.ole", tc.slug));
        fs::write(&ole_path, &ole_bytes).expect("write ole file");

        // Verify standard OLE Compound File Binary (CFB v3) structure:
        // Magic header: D0 CF 11 E0 A1 B1 1A E1
        assert_eq!(
            &ole_bytes[0..8],
            &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            "OLE magic bytes must be standard CFB D0CF11E0A1B11AE1"
        );
        assert_eq!(
            ole_bytes.len() % 512,
            0,
            "OLE size must be aligned to 512-byte sectors"
        );

        // Read Equation Native stream inside OLE
        let read_native = read_ole_stream(&ole_bytes, &["Equation Native", "contents"])
            .expect("must find Equation Native stream inside OLE");
        // Standard Equation Native header is 28 bytes: [0x1C, 0x00, 0x00, 0x00, 0x02, 0x00, ...]
        assert_eq!(
            read_native[0..4],
            [0x1C, 0x00, 0x00, 0x00],
            "Equation Native header size = 28 bytes"
        );
        assert_eq!(
            read_native[4..6],
            [0x02, 0x00],
            "Format type = 2 (embedded)"
        );

        // Extract MTEF stream from OLE and decode back to MathML
        let extracted_mtef = extract_mtef_from_ole(&ole_bytes).expect("extract MTEF from OLE");
        assert!(
            extracted_mtef[0] == 3 || extracted_mtef[0] == 5,
            "MTEF version byte must be 3 or 5"
        );
        let back_from_ole = mtef_to_mathml(&extracted_mtef).expect("decode MTEF to MathML");
        assert!(
            !back_from_ole.children.is_empty(),
            "decoded MTEF from OLE must not be empty"
        );
        println!(
            "  [OK] OLE Compound: {} bytes (512B sector-aligned), magic D0CF11E0A1B11AE1, 28B Equation Native header, MTEF v{} decode succeeded",
            ole_bytes.len(),
            extracted_mtef[0]
        );

        // 5. Export raster PNG image
        let preview = render_preview(&math, 12.0)
            .unwrap_or_else(|e| panic!("Render preview PNG failed for {}: {e}", tc.slug));
        let png_path = out_dir.join(format!("{}.png", tc.slug));
        fs::write(&png_path, &preview.png).expect("write png file");
        assert!(png_path.exists());
        // Verify PNG format:
        // 8-byte magic header: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(
            &preview.png[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
            "PNG magic header must be 89 50 4E 47 0D 0A 1A 0A"
        );
        // Check IHDR chunk
        assert_eq!(&preview.png[12..16], b"IHDR", "First chunk must be IHDR");
        // Check trailing IEND chunk
        assert_eq!(
            &preview.png[preview.png.len() - 8..preview.png.len() - 4],
            b"IEND",
            "Last chunk must be IEND"
        );
        assert!(
            preview.width_pt > 0.0 && preview.height_pt > 0.0,
            "Image dimensions width_pt and height_pt must be > 0"
        );
        println!(
            "  [OK] PNG Image: {} bytes, size {:.1}x{:.1} pt, valid PNG header + IHDR + IEND",
            preview.png.len(),
            preview.width_pt,
            preview.height_pt
        );

        // 6. Export vector EMF (Enhanced Metafile) with embedded MTEF
        let complete_mtef = build_mtef(&mtef_bytes);
        let emf_preview = render_emf_with_mtef(&math, 12.0, Some(&complete_mtef))
            .unwrap_or_else(|e| panic!("Render EMF failed for {}: {e}", tc.slug));
        let emf_path = out_dir.join(format!("{}.emf", tc.slug));
        fs::write(&emf_path, &emf_preview.emf).expect("write emf file");
        assert!(emf_path.exists());

        // Verify standard EMF format:
        // Record 1 Type = 0x00000001 (EMR_HEADER)
        let rec_type = u32::from_le_bytes(emf_preview.emf[0..4].try_into().unwrap());
        assert_eq!(rec_type, 1, "EMF Record 1 must be EMR_HEADER (type 1)");
        // Signature at bytes 40..44 must be " EMF" (0x464D4520)
        let sig = &emf_preview.emf[40..44];
        assert_eq!(sig, b" EMF", "EMF signature at offset 40 must be ' EMF'");

        // Verify extracting MTEF from EMF (preserves equation for Office / WPS)
        let emf_mtef =
            extract_mtef_from_emf(&emf_preview.emf).expect("extract embedded MTEF from EMF");
        assert_eq!(
            emf_mtef, complete_mtef,
            "MTEF extracted from EMR_COMMENT in EMF must match original MTEF 100%"
        );
        println!(
            "  [OK] EMF Vector: {} bytes, valid EMR_HEADER (' EMF'), embedded MTEF in EMR_COMMENT matches 100%",
            emf_preview.emf.len()
        );
    }

    println!("\n=== ALL 6 SAMPLES EXPORTED AND VERIFIED SUCCESSFULLY ===");
}

#[test]
fn test_export_visual_layout_sheets() {
    let out_dir = Path::new("target/test_exports");
    fs::create_dir_all(out_dir).expect("create test_exports dir");

    let sheets = [
        (
            "07_fractions_scripts",
            r"\dfrac{a}{b} + \tfrac{1}{2} + \sfrac{3}{4} + \tsfrac{5}{6} + \atopfrac{n}{k} + \sqrt[3]{x^2+1} + \prescript{235}{92}{U} + \overset{12}{\enclose{longdiv}{36}}",
        ),
        (
            "08_fences_all",
            r"\left\{\frac{a}{b}\right\} + \left\lfloor\frac{x}{2}\right\rfloor + \left\lceil\frac{y}{3}\right\rceil + \left\|\frac{u}{v}\right\| + \left\llbracket \frac{1}{2} \right. + \left. \frac{3}{4} \right\rrbracket + \overbrace{a+b}^{n} + \underbrace{c+d}_{m}",
        ),
        (
            "09_bigops_integrals",
            r"\iint\limits_{D} f + \iiint_{V} g + \oint\limits_{C} h + \sum\limits_{i=1}^{n} i + \sum\nolimits_{i=1}^{n} i + \prod\limits_{k=1}^{m} x_k + \bigcup\limits_{i\in I} A_i",
        ),
        (
            "10_matrices_variants",
            r"\begin{Bmatrix} a & b \\ c & d \end{Bmatrix} + \begin{Vmatrix} 1 & 2 \\ 3 & 4 \end{Vmatrix} + \left\{ \begin{array}{lcr} x & = & 100 \\ y+z & = & 2 \end{array} \right.",
        ),
        (
            "11_embellishments_bars",
            r"\dot{x} + \ddot{x} + \dddot{x} + \ddddot{x} + \underdot{x} + \underddot{x} + \hat{x} + \widehat{ABC} + \tilde{x} + \vec{v} + \doubleoverline{ab} + \doubleunderline{cd} + \overparen{AB}",
        ),
        (
            "12_arrows_enclosures",
            r"A \overset{f}{\underset{g}{\leftrightarrow}} B \overset{k_1}{\underset{k_{-1}}{\rightleftharpoons}} C + \enclose{roundedbox}{x+y} + \xcancel{z} + \enclose{top left}{a} + \enclose{top bottom left}{b} + \enclose{actuarial}{n}",
        ),
        (
            "13_symbols_greek_spaces",
            r"\mathbb{R}, \mathcal{L}, \mathfrak{g}, \varepsilon \varphi \vartheta \varpi \varrho \varsigma \varkappa \digamma \varTheta \varUpsilon \lambdabar \quad x\,y\:z\;w",
        ),
    ];

    for (slug, latex) in sheets {
        let math = parse_latex_to_mathml(latex)
            .unwrap_or_else(|e| panic!("Parse failed for {slug}: {e}\n{latex}"));
        let preview =
            render_preview(&math, 14.0).unwrap_or_else(|e| panic!("Render failed for {slug}: {e}"));
        let path = out_dir.join(format!("{slug}.png"));
        fs::write(&path, &preview.png).expect("write png");
        println!(
            "[OK] {slug}.png: {} bytes ({:.1}x{:.1} pt)",
            preview.png.len(),
            preview.width_pt,
            preview.height_pt
        );
    }
}

#[test]
fn test_export_all_catalog_pngs() {
    fn fill_x(n: &mut MathNode) {
        if matches!(n.tag.as_str(), "mrow" | "mtd") && n.children.is_empty() && n.text.is_empty() {
            n.children.push(MathNode::leaf("mi", "x"));
        }
        for child in &mut n.children {
            fill_x(child);
        }
    }
    fn node_with_attr(tag: &str, k: &str, v: &str, children: Vec<MathNode>) -> MathNode {
        let mut n = MathNode::with_children(tag, children);
        n.set_attr(k, v);
        n
    }

    let catalog_dir = Path::new("target/test_exports/catalog");
    let items_dir = catalog_dir.join("items");
    fs::create_dir_all(&items_dir).expect("create catalog/items dir");

    let mut total = 0usize;
    for (g_idx, (group_id, _group_label)) in GROUPS.iter().enumerate() {
        let group_entries: Vec<_> = ENTRIES.iter().filter(|e| e.group == *group_id).collect();
        let group_item_dir = items_dir.join(format!("{:02}_{group_id}", g_idx + 1));
        fs::create_dir_all(&group_item_dir).expect("create group item dir");

        let is_symbols_only = group_entries.iter().all(|e| !e.source.contains("#?"));
        let cols = if is_symbols_only { 4usize } else { 2usize };
        let per_page = if is_symbols_only { 24usize } else { 10usize };
        let sheet_pt = if is_symbols_only { 10.0 } else { 8.0 };

        for (page_idx, page_entries) in group_entries.chunks(per_page).enumerate() {
            let mut rows: Vec<MathNode> = Vec::new();
            for chunk in page_entries.chunks(cols) {
                let mut cells: Vec<MathNode> = Vec::new();
                for entry in chunk {
                    let preview_math = preview_template(entry.source)
                        .unwrap_or_else(|e| panic!("preview_template failed {}: {e}", entry.id));
                    let preview_png = render_preview(&preview_math, 14.0)
                        .unwrap_or_else(|e| panic!("render preview failed {}: {e}", entry.id));
                    assert!(!preview_png.png.is_empty());

                    let raw_node = template(entry.source)
                        .unwrap_or_else(|e| panic!("template failed {}: {e}", entry.id));
                    let raw_math = MathNode::with_children("math", vec![raw_node.clone()]);
                    let raw_png = render_preview(&raw_math, 14.0)
                        .unwrap_or_else(|e| panic!("render raw failed {}: {e}", entry.id));

                    let mut filled_node = raw_node;
                    fill_x(&mut filled_node);
                    let filled_math = MathNode::with_children("math", vec![filled_node.clone()]);
                    let filled_png = render_preview(&filled_math, 14.0)
                        .unwrap_or_else(|e| panic!("render filled failed {}: {e}", entry.id));

                    let safe_id: String = entry
                        .id
                        .chars()
                        .map(|c| {
                            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                                c
                            } else {
                                '_'
                            }
                        })
                        .collect();
                    fs::write(
                        group_item_dir.join(format!("{safe_id}__preview.png")),
                        &preview_png.png,
                    )
                    .expect("write item preview png");
                    if entry.source.contains("#?") {
                        fs::write(
                            group_item_dir.join(format!("{safe_id}__raw.png")),
                            &raw_png.png,
                        )
                        .expect("write item raw png");
                        fs::write(
                            group_item_dir.join(format!("{safe_id}__filled.png")),
                            &filled_png.png,
                        )
                        .expect("write item filled png");
                    }

                    let short_id = entry
                        .id
                        .strip_prefix("symbol-")
                        .or_else(|| entry.id.strip_prefix(&format!("{group_id}-")))
                        .unwrap_or(entry.id);
                    let mut cell_children = vec![
                        MathNode::leaf("mtext", &format!("[{short_id}]")),
                        node_with_attr("mspace", "width", "0.30em", vec![]),
                        node_with_attr("mstyle", "displaystyle", "true", preview_math.children),
                    ];
                    if entry.source.contains("#?") {
                        cell_children.push(node_with_attr("mspace", "width", "0.35em", vec![]));
                        cell_children.push(node_with_attr(
                            "mstyle",
                            "displaystyle",
                            "true",
                            vec![filled_node],
                        ));
                    }
                    let td = node_with_attr(
                        "mtd",
                        "columnalign",
                        "left",
                        vec![MathNode::with_children("mrow", cell_children)],
                    );
                    cells.push(td);
                    total += 1;
                }
                rows.push(MathNode::with_children("mtr", cells));
            }

            let sheet_math =
                MathNode::with_children("math", vec![MathNode::with_children("mtable", rows)]);
            let sheet_png = render_preview(&sheet_math, sheet_pt).unwrap_or_else(|e| {
                panic!("render sheet failed {group_id} p{}: {e}", page_idx + 1)
            });
            let sheet_path = if group_entries.len() > per_page {
                catalog_dir.join(format!("{:02}_{group_id}_p{}.png", g_idx + 1, page_idx + 1))
            } else {
                catalog_dir.join(format!("{:02}_{group_id}.png", g_idx + 1))
            };
            fs::write(&sheet_path, &sheet_png.png).expect("write group sheet png");
            println!(
                "[OK] {}: {} entries ({:.1}x{:.1} pt)",
                sheet_path.file_name().unwrap().to_string_lossy(),
                page_entries.len(),
                sheet_png.width_pt,
                sheet_png.height_pt
            );
        }
    }
    println!(
        "[OK] Exported {total} catalog entries across {} groups",
        GROUPS.len()
    );
}
