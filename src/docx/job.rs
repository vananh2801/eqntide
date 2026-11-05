//! One conversion run: locating equations in the document tree and turning
//! each into an OLE object (plus preview picture).

use std::collections::{HashSet};



use crate::error::{Error, Result};
use crate::mtef::encode::mathml_to_mtef_bytes;
use crate::mtef::make_equation_native;
use crate::ole::{make_ole};
use crate::omml::decode::{omml_to_mathml, NS_M};
use crate::render::emf::render_emf;
use crate::render::render_preview;
use crate::xml::{Element, Node};

use super::consts::*;
use super::options::*;
use super::ole_xml::*;
use super::package::*;

/// The preview picture of one equation, in whichever format was chosen.
struct Picture {
    data: Vec<u8>,
    ext: &'static str,
    content_type: &'static str,
    width_pt: f32,
    height_pt: f32,
    depth_pt: f32,
}


// ── Tree helpers ──────────────────────────────────────────────────────────────

/// Document-order paths (child indices) of every `m:oMath` below `e`.
///
/// Nested `m:oMath` elements are not collected (the schema forbids them, and
/// the outer equation already covers their content).
pub(super) fn collect_omath_paths(e: &Element, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if e.is(NS_M, "oMath") {
        out.push(path.clone());
        return;
    }
    for (i, n) in e.children.iter().enumerate() {
        if let Node::Element(c) = n {
            path.push(i);
            collect_omath_paths(c, path, out);
            path.pop();
        }
    }
}

pub(super) fn element_at<'a>(root: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut cur = root;
    for &i in path {
        match cur.children.get(i)? {
            Node::Element(c) => cur = c,
            _ => return None,
        }
    }
    Some(cur)
}

pub(super) fn element_at_mut<'a>(root: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    let mut cur = root;
    for &i in path {
        match cur.children.get_mut(i)? {
            Node::Element(c) => cur = c,
            _ => return None,
        }
    }
    Some(cur)
}

// ── Conversion state ──────────────────────────────────────────────────────────

pub(super) enum Outcome {
    Converted,
    Skipped,
}

pub(super) struct Job {
    pub(super) doc_root: Element,
    pub(super) rels_root: Element,
    pub(super) ct_root: Element,
    pub(super) pkg: Package,
    pub(super) existing_ids: HashSet<String>,
    pub(super) ole_index: usize,
    pub(super) img_index: usize,
    pub(super) opts: Options,
    pub(super) summary: ConversionSummary,
}

impl Job {
    pub(super) fn new_rel_id(&mut self) -> String {
        let mut i = 1;
        while self.existing_ids.contains(&format!("rIdOle{i}")) {
            i += 1;
        }
        let rid = format!("rIdOle{i}");
        self.existing_ids.insert(rid.clone());
        rid
    }

    /// Make `wanted` unique among the relationship ids (appending digits if needed).
    pub(super) fn unique_id(&mut self, wanted: String) -> String {
        let mut id = wanted.clone();
        let mut n = 0;
        while self.existing_ids.contains(&id) {
            n += 1;
            id = format!("{wanted}_{n}");
        }
        self.existing_ids.insert(id.clone());
        id
    }

    pub(super) fn convert_one(&mut self, eq_num: usize, path: &[usize]) -> Result<Outcome> {
        let omath = match element_at(&self.doc_root, path) {
            Some(e) if e.is(NS_M, "oMath") => e.clone(),
            _ => return Err(Error::Xml("equation node is no longer in the tree".into())),
        };

        // 1. OMML → MathML
        let mathml = omml_to_mathml(&omath)?;
        // Print MathML on terminal for debug review
        println!("{:#?}", mathml);

        // 2. MathML → MTEF binary
        let eq_bytes = mathml_to_mtef_bytes(&mathml)?;
        if eq_bytes.is_empty() {
            self.summary.skipped += 1;
            log::warn!("Equation {eq_num}: empty content, skipping");
            return Ok(Outcome::Skipped);
        }

        // 3. MTEF → Equation Native stream
        let native_stream = make_equation_native(&eq_bytes);

        // 4. Equation Native stream → OLE compound file
        let ole_bin = make_ole(&native_stream)?;

        // 4b. Preview picture (Word/WPS show the object through it).
        let preview = match self.opts.mode {
            EmbedMode::Preview => Some(match self.opts.preview {
                PreviewFormat::Png => {
                    let p = render_preview(&mathml, self.opts.font_pt)?;
                    Picture {
                        data: p.png,
                        ext: "png",
                        content_type: "image/png",
                        width_pt: p.width_pt,
                        height_pt: p.height_pt,
                        depth_pt: p.depth_pt,
                    }
                }
                PreviewFormat::Emf => {
                    let p = render_emf(&mathml, self.opts.font_pt)?;
                    Picture {
                        data: p.emf,
                        ext: "emf",
                        content_type: "image/x-emf",
                        width_pt: p.width_pt,
                        height_pt: p.height_pt,
                        depth_pt: p.depth_pt,
                    }
                }
            }),
            EmbedMode::OmmlFallback => None,
        };

        // Decide what gets replaced. m:oMath may live directly in w:p, or be
        // wrapped by m:oMathPara; in the latter case we replace the whole
        // wrapper so no empty orphan oMathPara is left behind.
        let Some((&omath_idx, parent_path)) = path.split_last() else {
            return Err(Error::Xml("equation is the document root".into()));
        };
        let parent = element_at(&self.doc_root, parent_path)
            .ok_or_else(|| Error::Xml("equation has no parent".into()))?;
        let target_path: &[usize] = if parent.local == "oMathPara" {
            if parent.find_all(NS_M, "oMath").len() > 1 {
                // Replacing the wrapper would silently drop its other equations.
                self.summary.skipped += 1;
                log::warn!(
                    "Equation {eq_num}: oMathPara contains several equations; left as OMML"
                );
                return Ok(Outcome::Skipped);
            }
            if parent_path.is_empty() {
                return Err(Error::Xml("oMathPara is the document root".into()));
            }
            parent_path
        } else {
            path
        };
        let _ = omath_idx;

        // 5. Store the .bin in the docx embeddings
        let bin_name = format!("word/embeddings/oleObject{}.bin", self.ole_index);
        self.pkg.set(&bin_name, ole_bin);
        let target = format!("embeddings/oleObject{}.bin", self.ole_index);
        self.ole_index += 1;

        // 6. Add relationship
        let rel_id = self.new_rel_id();
        add_relationship(&mut self.rels_root, &rel_id, REL_OLEOBJECT, &target);

        // 7. Add content type for .bin
        ensure_content_type(&mut self.ct_root, &format!("/{bin_name}"), CT_OLEOBJECT);

        // 8. Replace the equation node in the document tree.
        let shape_id = format!("_x0000_i{}", 1024 + eq_num);
        let alt = match &preview {
            Some(pv) => {
                // Bare MathType object + preview picture, as Word writes it.
                let mut img_name = format!("word/media/equation{}.{}", self.img_index, pv.ext);
                while self.pkg.contains(&img_name) {
                    self.img_index += 1;
                    img_name = format!("word/media/equation{}.{}", self.img_index, pv.ext);
                }
                self.img_index += 1;
                self.pkg.set(&img_name, pv.data.clone());
                let img_rid = format!("rIdImg{}", self.img_index - 1);
                let img_rid = self.unique_id(img_rid);
                add_relationship(
                    &mut self.rels_root,
                    &img_rid,
                    REL_IMAGE,
                    &img_name["word/".len()..],
                );
                ensure_default_extension(&mut self.ct_root, pv.ext, pv.content_type);
                let obj_id = format!("_{}", 1_000_000_000u128 + uuid::Uuid::new_v4().as_u128() % 9_000_000_000u128);
                make_object_run(&rel_id, &img_rid, &shape_id, &obj_id, pv.width_pt, pv.height_pt, pv.depth_pt)
            }
            None => {
                let obj_id = format!("{{{}}}", uuid::Uuid::new_v4().to_string().to_uppercase());
                make_ole_xml(
                    &rel_id,
                    &shape_id,
                    &obj_id,
                    914_400, // 1 inch wide  — default display size
                    457_200, // 0.5 inch tall
                    &omath,  // original equation kept as visible fallback
                )
            }
        };

        let (&slot, holder_path) = target_path
            .split_last()
            .expect("target path is non-empty here");
        let holder = element_at_mut(&mut self.doc_root, holder_path)
            .ok_or_else(|| Error::Xml("equation has no parent".into()))?;
        holder.children[slot] = Node::Element(alt);

        self.summary.converted += 1;
        log::info!("Equation {eq_num}: converted → {bin_name}");
        Ok(Outcome::Converted)
    }
}
