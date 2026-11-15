use std::fs;
use std::path::Path;

use eqntide::mtef::decode::mtef_to_mathml;
use eqntide::mtef::encode::mathml_to_mtef_bytes;
use eqntide::mtef::{build_mtef, make_equation_native};
use eqntide::ole::{extract_mtef_from_emf, extract_mtef_from_ole, make_ole, read_ole_stream};
use eqntide::omml::{mathml_to_omml_xml, omml_to_mathml};
use eqntide::render::emf::render_emf_with_mtef;
use eqntide::render::render_preview;
use eqntide::service::parse_latex_to_mathml;
use eqntide::xml;

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
            title: "Phân số & Căn bậc hai (Quadratic Formula)",
            latex: r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        },
        TestCase {
            slug: "02_integral_limits",
            title: "Tích phân có cận trên/dưới (Definite Integral)",
            latex: r"\int\limits_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}",
        },
        TestCase {
            slug: "03_dirac_interval",
            title: "Ngoặc Dirac & khoảng nửa mở (Dirac Bra-Ket & Interval)",
            latex: r"\langle \psi | \phi \rangle + \left[ a, b \right)",
        },
        TestCase {
            slug: "04_matrix_3x3",
            title: "Ma trận 3x3 có ngoặc tròn (3x3 Matrix)",
            latex: r"\begin{pmatrix} 1 & 0 & 0 \\ 0 & 1 & 0 \\ 0 & 0 & 1 \end{pmatrix}",
        },
        TestCase {
            slug: "05_labeled_arrow_box",
            title: "Mũi tên có nhãn & Khung gạch xoá (Labeled Arrow & Boxed Cancel)",
            latex: r"A \overset{f}{\rightarrow} B + \boxed{x} + \cancel{y}",
        },
        TestCase {
            slug: "06_double_bracket_bar",
            title: "Ngoặc vuông kép & Gạch kép trên (Double Bracket & Overbar)",
            latex: r"\left\llbracket \frac{x}{y} \right\rrbracket + \doubleoverline{z}",
        },
    ];

    println!("\n=== BẮT ĐẦU KIỂM THỬ XUẤT ĐỊNH DẠNG (MathML, OMML, OLE/MTEF, PNG, EMF) ===");

    for tc in &test_cases {
        println!("\n------------------------------------------------------------");
        println!("Mẫu: {} ({})", tc.title, tc.slug);
        println!("LaTeX: {}", tc.latex);

        // 1. Chuyển LaTeX -> MathML
        let math = parse_latex_to_mathml(tc.latex)
            .unwrap_or_else(|e| panic!("Parse LaTeX thất bại cho {}: {e}", tc.slug));
        let mathml_xml = math.to_xml();

        // 2. Xuất MathML XML
        let mathml_path = out_dir.join(format!("{}.mathml", tc.slug));
        fs::write(&mathml_path, mathml_xml.as_bytes()).expect("ghi file mathml");
        assert!(mathml_path.exists());
        let read_mathml = fs::read_to_string(&mathml_path).expect("đọc lại mathml");
        let parsed_xml = xml::parse(read_mathml.as_bytes()).expect("XML MathML hợp lệ");
        assert_eq!(parsed_xml.local, "math");
        println!("  [OK] MathML: {} bytes, root=<math>", read_mathml.len());

        // 3. Xuất Word OMML XML
        let omml_xml = mathml_to_omml_xml(&math)
            .unwrap_or_else(|e| panic!("Sinh OMML thất bại cho {}: {e}", tc.slug));
        let omml_path = out_dir.join(format!("{}.omml.xml", tc.slug));
        fs::write(&omml_path, omml_xml.as_bytes()).expect("ghi file omml");
        assert!(omml_path.exists());
        let read_omml = fs::read_to_string(&omml_path).expect("đọc lại omml");
        let parsed_omml_el = xml::parse(read_omml.as_bytes()).expect("XML OMML hợp lệ");
        assert!(
            parsed_omml_el.local == "oMath" || parsed_omml_el.local == "oMathPara",
            "OMML local name phải là oMath hoặc oMathPara, thực tế: {}",
            parsed_omml_el.local
        );
        // Round-trip OMML -> MathML
        let back_from_omml = omml_to_mathml(&parsed_omml_el).expect("Giải mã OMML về MathML");
        assert!(
            !back_from_omml.children.is_empty(),
            "OMML decode không được rỗng"
        );
        println!(
            "  [OK] OMML: {} bytes, namespace m:oMath hợp lệ, round-trip OMML -> MathML thành công",
            read_omml.len()
        );

        // 4. Xuất OLE binary chứa MathType Equation Native Stream (MTEF)
        let mtef_bytes = mathml_to_mtef_bytes(&math)
            .unwrap_or_else(|e| panic!("MTEF encoding thất bại cho {}: {e}", tc.slug));
        assert!(!mtef_bytes.is_empty(), "MTEF bytes không được rỗng");
        let native_stream = make_equation_native(&mtef_bytes);
        let ole_bytes = make_ole(&native_stream)
            .unwrap_or_else(|e| panic!("Tạo OLE compound thất bại cho {}: {e}", tc.slug));
        let ole_path = out_dir.join(format!("{}.ole", tc.slug));
        fs::write(&ole_path, &ole_bytes).expect("ghi file ole");

        // Kiểm tra cấu trúc chuẩn OLE Compound File Binary (CFB v3):
        // Magic header: D0 CF 11 E0 A1 B1 1A E1
        assert_eq!(
            &ole_bytes[0..8],
            &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            "OLE Magic bytes phải là CFB standard D0CF11E0A1B11AE1"
        );
        assert_eq!(
            ole_bytes.len() % 512,
            0,
            "Kích thước OLE phải chia hết cho sector 512 bytes"
        );

        // Đọc stream Equation Native bên trong OLE
        let read_native = read_ole_stream(&ole_bytes, &["Equation Native", "contents"])
            .expect("Phải tìm thấy stream Equation Native trong OLE");
        // Header của Equation Native chuẩn gồm 28 bytes: [0x1C, 0x00, 0x00, 0x00, 0x02, 0x00, ...]
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

        // Trích xuất MTEF stream từ OLE và giải mã ngược về MathML
        let extracted_mtef = extract_mtef_from_ole(&ole_bytes).expect("Trích xuất MTEF từ OLE");
        assert!(
            extracted_mtef[0] == 3 || extracted_mtef[0] == 5,
            "MTEF version byte phải là 3 hoặc 5"
        );
        let back_from_ole = mtef_to_mathml(&extracted_mtef).expect("MTEF decode về MathML");
        assert!(
            !back_from_ole.children.is_empty(),
            "MTEF từ OLE decode không được rỗng"
        );
        println!(
            "  [OK] OLE Compound: {} bytes (sector 512-aligned), magic D0CF11E0A1B11AE1, Equation Native 28B header, MTEF v{} decode thành công",
            ole_bytes.len(),
            extracted_mtef[0]
        );

        // 5. Xuất ảnh raster PNG
        let preview = render_preview(&math, 12.0)
            .unwrap_or_else(|e| panic!("Render preview PNG thất bại cho {}: {e}", tc.slug));
        let png_path = out_dir.join(format!("{}.png", tc.slug));
        fs::write(&png_path, &preview.png).expect("ghi file png");
        assert!(png_path.exists());
        // Kiểm tra định dạng PNG:
        // Magic header 8 bytes: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(
            &preview.png[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
            "PNG Magic header phải là 89 50 4E 47 0D 0A 1A 0A"
        );
        // Kiểm tra chunk IHDR
        assert_eq!(&preview.png[12..16], b"IHDR", "Chunk đầu tiên phải là IHDR");
        // Kiểm tra chunk IEND ở cuối
        assert_eq!(
            &preview.png[preview.png.len() - 8..preview.png.len() - 4],
            b"IEND",
            "Chunk cuối cùng phải là IEND"
        );
        assert!(
            preview.width_pt > 0.0 && preview.height_pt > 0.0,
            "Kích thước hình ảnh width_pt, height_pt phải > 0"
        );
        println!(
            "  [OK] PNG Image: {} bytes, kích thước {:.1}x{:.1} pt, header PNG + IHDR + IEND chuẩn",
            preview.png.len(),
            preview.width_pt,
            preview.height_pt
        );

        // 6. Xuất ảnh vector EMF (Enhanced Metafile) có nhúng MTEF
        let complete_mtef = build_mtef(&mtef_bytes);
        let emf_preview = render_emf_with_mtef(&math, 12.0, Some(&complete_mtef))
            .unwrap_or_else(|e| panic!("Render EMF thất bại cho {}: {e}", tc.slug));
        let emf_path = out_dir.join(format!("{}.emf", tc.slug));
        fs::write(&emf_path, &emf_preview.emf).expect("ghi file emf");
        assert!(emf_path.exists());

        // Kiểm tra định dạng EMF chuẩn:
        // Record 1 Type = 0x00000001 (EMR_HEADER)
        let rec_type = u32::from_le_bytes(emf_preview.emf[0..4].try_into().unwrap());
        assert_eq!(rec_type, 1, "EMF Record 1 phải là EMR_HEADER (type 1)");
        // Signature tại byte 40..44 phải là " EMF" (0x464D4520)
        let sig = &emf_preview.emf[40..44];
        assert_eq!(sig, b" EMF", "EMF Signature tại offset 40 phải là ' EMF'");

        // Kiểm tra trích xuất MTEF từ EMF (bảo toàn công thức cho Office / WPS)
        let emf_mtef =
            extract_mtef_from_emf(&emf_preview.emf).expect("Trích xuất MTEF nhúng trong EMF");
        assert_eq!(
            emf_mtef, complete_mtef,
            "MTEF trích xuất từ EMR_COMMENT trong EMF phải khớp 100% với MTEF gốc"
        );
        println!(
            "  [OK] EMF Vector: {} bytes, EMR_HEADER chuẩn (' EMF'), nhúng MTEF trong EMR_COMMENT khớp 100%",
            emf_preview.emf.len()
        );
    }

    println!("\n=== TẤT CẢ 6 MẪU ĐÃ XUẤT VÀ KIỂM ĐỊNH THÀNH CÔNG ===");
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
