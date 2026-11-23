use std::io::{Cursor, Write};
use std::time::Instant;

use eqntide::docx::{
    convert_bytes_to_omml_with, convert_bytes_with, upgrade_legacy_mathtype_bytes_with, Options,
    PreviewFormat, ToOmmlOptions, UpgradeOptions, UpgradeTarget,
};
use eqntide::omml::encode::mathml_to_omml_xml;
use eqntide::service::parse_latex_to_mathml;

const SAMPLE_LATEX: &[&str] = &[
    r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
    r"\int_0^\infty e^{-x^2} \, dx = \frac{\sqrt{\pi}}{2}",
    r"\sum_{k=1}^{n} k^2 = \frac{n(n+1)(2n+1)}{6}",
    r"\alpha_i + \beta_{i,j} \le \gamma^{k+1}",
    r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
    r"\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{pmatrix} x \\ y \end{pmatrix}",
];

fn make_omml_docx(equation_count: usize) -> Vec<u8> {
    let sample_ommls: Vec<String> = SAMPLE_LATEX
        .iter()
        .map(|l| {
            let mml = parse_latex_to_mathml(l).expect("valid sample latex");
            mathml_to_omml_xml(&mml).expect("valid omml")
        })
        .collect();

    let mut body = String::with_capacity(equation_count * 512);
    for i in 0..equation_count {
        body.push_str("<w:p><w:r><w:t>Eq </w:t></w:r>");
        body.push_str(&sample_ommls[i % sample_ommls.len()]);
        body.push_str("</w:p>");
    }

    let doc_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>{body}</w:body>
</w:document>"#
    );
    let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    w.start_file("[Content_Types].xml", opts).unwrap();
    w.write_all(ct_xml.as_bytes()).unwrap();
    w.start_file("word/document.xml", opts).unwrap();
    w.write_all(doc_xml.as_bytes()).unwrap();
    w.finish().unwrap().into_inner()
}

#[test]
#[ignore = "performance benchmark: run manually with cargo test --release --test perf_benchmark -- --ignored --nocapture"]
fn perf_single_large_and_batch_workloads() {
    // Warm up OnceLock<Fonts> and Rayon pool
    let warmup_docx = make_omml_docx(4);
    let _ = convert_bytes_with(&warmup_docx, &Options::default()).unwrap();

    // 1. Single large document: 600 equations
    let large_omml_docx = make_omml_docx(600);

    let t = Instant::now();
    let (mt_emf_docx, sum_emf) = convert_bytes_with(
        &large_omml_docx,
        &Options {
            preview: PreviewFormat::Emf,
            ..Options::default()
        },
    )
    .unwrap();
    let dt_emf = t.elapsed();
    assert_eq!(sum_emf.converted, 600);

    let t = Instant::now();
    let (mt_png_docx, sum_png) = convert_bytes_with(
        &large_omml_docx,
        &Options {
            preview: PreviewFormat::Png,
            ..Options::default()
        },
    )
    .unwrap();
    let dt_png = t.elapsed();
    assert_eq!(sum_png.converted, 600);

    let t = Instant::now();
    let (_omml_back, sum_to_omml) =
        convert_bytes_to_omml_with(&mt_emf_docx, &ToOmmlOptions::default()).unwrap();
    let dt_to_omml = t.elapsed();
    assert_eq!(sum_to_omml.converted, 600);

    let t = Instant::now();
    let (_upgraded, sum_upg) = upgrade_legacy_mathtype_bytes_with(
        &mt_emf_docx,
        &UpgradeOptions {
            target: UpgradeTarget::ModernMathType,
            ..UpgradeOptions::default()
        },
    )
    .unwrap();
    let dt_upg = t.elapsed();
    assert_eq!(sum_upg.converted, 600);

    eprintln!(
        "[Single 600 eqs] OMML->MT(EMF)={:.1}ms (size={}KB) | OMML->MT(PNG)={:.1}ms (size={}KB) | MT->OMML={:.1}ms | Upgrade->MT={:.1}ms",
        dt_emf.as_secs_f64() * 1e3,
        mt_emf_docx.len() / 1024,
        dt_png.as_secs_f64() * 1e3,
        mt_png_docx.len() / 1024,
        dt_to_omml.as_secs_f64() * 1e3,
        dt_upg.as_secs_f64() * 1e3,
    );

    // 2. Multi-file CLI directory batch: 64 files x 12 equations = 768 equations
    let temp_dir = std::env::temp_dir().join(format!("eqntide_perf_batch_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let in_dir = temp_dir.join("in");
    let out_dir = temp_dir.join("out");
    std::fs::create_dir_all(&in_dir).unwrap();
    let small_docx = make_omml_docx(12);
    for i in 0..64 {
        std::fs::write(in_dir.join(format!("doc_{i:02}.docx")), &small_docx).unwrap();
    }

    let t = Instant::now();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = eqntide::cli::run_with_io(
        [
            "eqntide-cli",
            in_dir.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--quiet",
        ],
        &mut stdout,
        &mut stderr,
    );
    let dt_batch = t.elapsed();
    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&stderr));
    eprintln!(
        "[CLI Batch 64x12=768 eqs] total={:.1}ms",
        dt_batch.as_secs_f64() * 1e3
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
