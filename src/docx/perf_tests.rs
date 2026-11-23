use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::time::{Duration, Instant};

use crate::docx::consts::*;
use crate::docx::job::{collect_omath_paths, Job};
use crate::docx::options::{ConversionSummary, Options, PreviewFormat};
use crate::docx::package::{collect_override_part_names, Package};
use crate::omml::encode::mathml_to_omml_xml;
use crate::service::parse_latex_to_mathml;
use crate::xml;

const SAMPLE_LATEX: &[&str] = &[
    r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
    r"\int_0^\infty e^{-x^2} \, dx = \frac{\sqrt{\pi}}{2}",
    r"\sum_{k=1}^{n} k^2 = \frac{n(n+1)(2n+1)}{6}",
    r"\alpha_i + \beta_{i,j} \le \gamma^{k+1}",
    r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
    r"\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{pmatrix} x \\ y \end{pmatrix}",
];

fn build_omml_docx(equation_count: usize) -> Vec<u8> {
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

fn measure_phases(
    input: &[u8],
    opts: &Options,
) -> (Duration, Duration, Duration, Duration, Duration) {
    let t0 = Instant::now();
    let pkg = Package::read(input).unwrap();
    let zip_read = t0.elapsed();

    let t1 = Instant::now();
    let ct_root = xml::parse(pkg.get(CT_PATH).unwrap()).unwrap();
    let doc_root = xml::parse(pkg.get(DOC_XML_PATH).unwrap()).unwrap();
    let rels_root = xml::parse(
        pkg.get("word/_rels/document.xml.rels")
            .unwrap_or(EMPTY_RELS),
    )
    .unwrap();
    let xml_parse = t1.elapsed();

    let mut paths = Vec::new();
    collect_omath_paths(&doc_root, &mut Vec::new(), &mut paths);
    let existing_ids: HashSet<String> = rels_root
        .elements()
        .filter_map(|c| c.attr(None, "Id").map(str::to_owned))
        .collect();

    let ct_overrides = collect_override_part_names(&ct_root);
    let mut job = Job {
        doc_root,
        rels_root,
        ct_root,
        ct_overrides,
        pkg,
        existing_ids,
        next_ole_rid: 1,
        next_img_rid: 1,
        ole_index: 1,
        img_index: 1,
        opts: *opts,
        summary: ConversionSummary::default(),
    };

    let t2 = Instant::now();
    let mut global_eq_num = 0;
    job.convert_part_equations(DOC_XML_PATH, &paths, &mut global_eq_num, None)
        .unwrap();
    super::ole_xml::sanitize_document_baselines(&mut job.doc_root);
    let compute_and_dom = t2.elapsed();

    let t3 = Instant::now();
    let doc_bytes = xml::to_bytes(&job.doc_root);
    let rels_bytes = xml::to_bytes(&job.rels_root);
    let ct_bytes = xml::to_bytes(&job.ct_root);
    let xml_serialize = t3.elapsed();

    let mut pkg = job.pkg;
    pkg.set(DOC_XML_PATH, doc_bytes);
    pkg.set("word/_rels/document.xml.rels", rels_bytes);
    pkg.set(CT_PATH, ct_bytes);

    let t4 = Instant::now();
    let _out = pkg.write().unwrap();
    let zip_write = t4.elapsed();

    (
        zip_read,
        xml_parse,
        compute_and_dom,
        xml_serialize,
        zip_write,
    )
}

#[test]
#[ignore = "performance benchmark: run manually with cargo test --release --lib docx::perf_tests -- --ignored --nocapture"]
fn perf_phase_breakdown_500_equations() {
    let docx = build_omml_docx(500);
    // Warm up OnceLock<Fonts>
    let _ = crate::render::font::fonts();

    for (label, preview) in [("EMF", PreviewFormat::Emf), ("PNG", PreviewFormat::Png)] {
        let opts = Options {
            preview,
            ..Options::default()
        };
        let (zip_r, xml_p, comp, xml_s, zip_w) = measure_phases(&docx, &opts);
        let total = zip_r + xml_p + comp + xml_s + zip_w;
        let pct = |d: Duration| 100.0 * d.as_secs_f64() / total.as_secs_f64();
        eprintln!(
            "[500 eqs {label}] total={:.1}ms | zip_read={:.1}ms ({:.1}%) | xml_parse={:.1}ms ({:.1}%) | compute+dom={:.1}ms ({:.1}%) | xml_ser={:.1}ms ({:.1}%) | zip_write={:.1}ms ({:.1}%)",
            total.as_secs_f64() * 1e3,
            zip_r.as_secs_f64() * 1e3, pct(zip_r),
            xml_p.as_secs_f64() * 1e3, pct(xml_p),
            comp.as_secs_f64() * 1e3, pct(comp),
            xml_s.as_secs_f64() * 1e3, pct(xml_s),
            zip_w.as_secs_f64() * 1e3, pct(zip_w),
        );
    }
}

#[cfg(target_os = "linux")]
fn proc_memory_kb() -> (Option<usize>, Option<usize>) {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return (None, None);
    };
    let mut rss = None;
    let mut hwm = None;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss = rest.split_whitespace().next().and_then(|s| s.parse().ok());
        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
            hwm = rest.split_whitespace().next().and_then(|s| s.parse().ok());
        }
    }
    (rss, hwm)
}

#[cfg(not(target_os = "linux"))]
fn proc_memory_kb() -> (Option<usize>, Option<usize>) {
    (None, None)
}

fn fmt_kb(kb: Option<usize>) -> String {
    kb.map(|v| format!("{:.1} MB", v as f64 / 1024.0))
        .unwrap_or_else(|| "n/a".into())
}

#[test]
#[ignore = "memory benchmark: run manually with cargo test --release --lib docx::perf_tests::perf_memory_omml_to_mathtype_3000 -- --ignored --nocapture"]
fn perf_memory_omml_to_mathtype_3000() {
    let _ = crate::render::font::fonts();
    let dir = std::env::temp_dir().join(format!("eqntide_mem_o2m_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let in_path = dir.join("omml_3000.docx");
    let out_path = dir.join("mt_3000.docx");
    std::fs::write(&in_path, build_omml_docx(3000)).unwrap();

    let (rss_before, _) = proc_memory_kb();
    let t0 = Instant::now();
    let summary = crate::docx::convert(&in_path, &out_path).unwrap();
    let dt = t0.elapsed();
    let (rss_after, hwm) = proc_memory_kb();
    assert_eq!(summary.converted, 3000);
    eprintln!(
        "[3000 eqs OMML->MT] elapsed={:.1}ms | rss_before={} | rss_after={} | peak_hwm={}",
        dt.as_secs_f64() * 1e3,
        fmt_kb(rss_before),
        fmt_kb(rss_after),
        fmt_kb(hwm),
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "memory benchmark: run manually with cargo test --release --lib docx::perf_tests::perf_memory_mathtype_to_omml_3000 -- --ignored --nocapture"]
fn perf_memory_mathtype_to_omml_3000() {
    let _ = crate::render::font::fonts();
    let dir = std::env::temp_dir().join(format!("eqntide_mem_m2o_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let omml_path = dir.join("omml_3000.docx");
    let mt_path = dir.join("mt_3000.docx");
    let out_path = dir.join("omml_back_3000.docx");
    std::fs::write(&omml_path, build_omml_docx(3000)).unwrap();
    crate::docx::convert(&omml_path, &mt_path).unwrap();

    let (rss_before, _) = proc_memory_kb();
    let t0 = Instant::now();
    let summary = crate::docx::convert_to_omml(&mt_path, &out_path).unwrap();
    let dt = t0.elapsed();
    let (rss_after, hwm) = proc_memory_kb();
    assert_eq!(summary.converted, 3000);
    eprintln!(
        "[3000 eqs MT->OMML] elapsed={:.1}ms | rss_before={} | rss_after={} | peak_hwm={}",
        dt.as_secs_f64() * 1e3,
        fmt_kb(rss_before),
        fmt_kb(rss_after),
        fmt_kb(hwm),
    );
    let _ = std::fs::remove_dir_all(&dir);
}
