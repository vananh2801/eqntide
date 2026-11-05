//! Debug helper: render the first N equations of a .docx to PNG files.
//! `cargo run --example render_docx -- in.docx out_dir [start] [count]`
use std::io::Read;

use omml_converter::omml::{omml_to_mathml, NS_M};
use omml_converter::{render, xml};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut z = zip::ZipArchive::new(std::fs::File::open(&a[1]).unwrap()).unwrap();
    let mut data = Vec::new();
    z.by_name("word/document.xml").unwrap().read_to_end(&mut data).unwrap();
    let root = xml::parse(&data).unwrap();
    let start: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let count: usize = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(40);
    std::fs::create_dir_all(&a[2]).unwrap();
    let eqs: Vec<_> = root.iter().into_iter().filter(|e| e.is(NS_M, "oMath") && !e.is_empty()).collect();
    println!("{} equations", eqs.len());
    for (i, e) in eqs.iter().enumerate().skip(start).take(count) {
        let m = omml_to_mathml(e).unwrap();
        // PNG
        match render::render_preview(&m, 12.0) {
            Ok(p) => std::fs::write(format!("{}/eq{:04}.png", a[2], i), &p.png).unwrap(),
            Err(err) => println!("eq {i}: {err}"),
        }
        // EMF
        match render::emf::render_emf(&m, 12.0) {
              Ok(p) => std::fs::write(format!("{}/eq{:04}.emf", a[2], i), &p.emf).unwrap(),
             Err(err) => println!("eq {i} (emf): {err}"),
        }
    }
}
