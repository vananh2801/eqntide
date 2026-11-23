//! One conversion run: locating equations in the document tree and turning
//! each into an OLE object (plus preview picture).

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::mtef::encode::mathml_to_mtef_bytes;
use crate::mtef::make_equation_native;
use crate::ole::make_ole;
use crate::omml::decode::{omml_to_mathml, NS_M};
use crate::render::emf::render_emf_with_mtef;
use crate::render::render_preview;
use crate::xml::{Element, Node};

use super::consts::*;
use super::ole_xml::*;
use super::options::*;
use super::package::*;

/// The preview picture of one equation, in whichever format was chosen.
pub(super) struct Picture {
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

pub(super) struct Job {
    pub(super) doc_root: Element,
    pub(super) rels_root: Element,
    pub(super) ct_root: Element,
    pub(super) ct_overrides: HashSet<String>,
    pub(super) pkg: Package,
    pub(super) existing_ids: HashSet<String>,
    pub(super) next_ole_rid: usize,
    pub(super) next_img_rid: usize,
    pub(super) ole_index: usize,
    pub(super) img_index: usize,
    pub(super) opts: Options,
    pub(super) summary: ConversionSummary,
}

const EQUATION_CHUNK_SIZE: usize = 128;

pub(super) struct EquationWorkItem<'a> {
    pub(super) eq_num: usize,
    pub(super) target_path: Vec<usize>,
    pub(super) omath: &'a Element,
}

pub(super) enum PreparedOutcome {
    Converted {
        ole_bin: Vec<u8>,
        preview: Option<Picture>,
        omath_fallback: Option<Box<Element>>,
    },
    Skipped(&'static str),
}

pub(super) enum CollectedItem {
    Skipped(&'static str, usize),
    Failed(usize, Error),
}

fn prepare_equation(
    omath: &Element,
    opts: Options,
    cancel: Option<&AtomicBool>,
) -> Result<PreparedOutcome> {
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }

    // 1. OMML → MathML
    let mathml = omml_to_mathml(omath)?;
    if let Some(reason) = crate::mtef::check_mathtype_unsupported(&mathml) {
        return Ok(PreparedOutcome::Skipped(reason));
    }

    // 2. MathML → MTEF binary
    let eq_bytes = mathml_to_mtef_bytes(&mathml)?;
    if eq_bytes.is_empty() {
        return Ok(PreparedOutcome::Skipped("empty content"));
    }

    // 3. MTEF → Equation Native stream
    let native_stream = make_equation_native(&eq_bytes);

    // 4. Equation Native stream → OLE compound file
    let ole_bin = make_ole(&native_stream)?;

    // 4b. Preview picture (Word/WPS show the object through it).
    let preview = match opts.mode {
        EmbedMode::Preview => Some(match opts.preview {
            PreviewFormat::Png => {
                let p = render_preview(&mathml, opts.font_pt)?;
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
                let complete_mtef = crate::mtef::build_mtef(&eq_bytes);
                let p = render_emf_with_mtef(&mathml, opts.font_pt, Some(&complete_mtef))?;
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

    let omath_fallback = if opts.mode == EmbedMode::OmmlFallback {
        Some(Box::new(omath.clone()))
    } else {
        None
    };

    Ok(PreparedOutcome::Converted {
        ole_bin,
        preview,
        omath_fallback,
    })
}

impl Job {
    pub(super) fn new_rel_id(&mut self) -> String {
        loop {
            let rid = format!("rIdOle{}", self.next_ole_rid);
            self.next_ole_rid += 1;
            if self.existing_ids.insert(rid.clone()) {
                return rid;
            }
        }
    }

    pub(super) fn new_img_rel_id(&mut self) -> String {
        loop {
            let rid = format!("rIdImg{}", self.next_img_rid);
            self.next_img_rid += 1;
            if self.existing_ids.insert(rid.clone()) {
                return rid;
            }
        }
    }

    /// Collect equations, run pure CPU conversion in bounded parallel chunks,
    /// and assemble DOM/package sequentially per chunk to cap peak memory.
    pub(super) fn convert_part_equations(
        &mut self,
        xml_path: &str,
        paths: &[Vec<usize>],
        global_eq_num: &mut usize,
        cancel: Option<&AtomicBool>,
    ) -> Result<()> {
        let opts = self.opts;
        for chunk_paths in paths.chunks(EQUATION_CHUNK_SIZE) {
            if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return Err(Error::Cancelled);
            }

            let mut collected: Vec<CollectedItem> = Vec::new();
            let mut metas: Vec<(usize, Vec<usize>)> = Vec::with_capacity(chunk_paths.len());
            let mut prepared_results: Vec<Result<PreparedOutcome>>;

            {
                let doc_ref = &self.doc_root;
                let mut work_items: Vec<EquationWorkItem<'_>> =
                    Vec::with_capacity(chunk_paths.len());

                for path in chunk_paths {
                    *global_eq_num += 1;
                    let eq_num = *global_eq_num;
                    let omath = match element_at(doc_ref, path) {
                        Some(e) if e.is(NS_M, "oMath") => e,
                        _ => {
                            collected.push(CollectedItem::Failed(
                                eq_num,
                                Error::Xml("equation node is no longer in the tree".into()),
                            ));
                            continue;
                        }
                    };

                    let Some((_, parent_path)) = path.split_last() else {
                        collected.push(CollectedItem::Failed(
                            eq_num,
                            Error::Xml("equation is the document root".into()),
                        ));
                        continue;
                    };
                    let Some(parent) = element_at(doc_ref, parent_path) else {
                        collected.push(CollectedItem::Failed(
                            eq_num,
                            Error::Xml("equation has no parent".into()),
                        ));
                        continue;
                    };
                    let target_path: Vec<usize> = if parent.local == "oMathPara" {
                        if parent.find_all(NS_M, "oMath").len() > 1 {
                            collected.push(CollectedItem::Skipped(
                                "oMathPara contains several equations; left as OMML",
                                eq_num,
                            ));
                            continue;
                        }
                        if parent_path.is_empty() {
                            collected.push(CollectedItem::Failed(
                                eq_num,
                                Error::Xml("oMathPara is the document root".into()),
                            ));
                            continue;
                        }
                        parent_path.to_vec()
                    } else {
                        path.clone()
                    };

                    work_items.push(EquationWorkItem {
                        eq_num,
                        target_path,
                        omath,
                    });
                }

                prepared_results = if work_items.len() <= 1 {
                    work_items
                        .iter()
                        .map(|item| prepare_equation(item.omath, opts, cancel))
                        .collect()
                } else {
                    work_items
                        .par_iter()
                        .map(|item| prepare_equation(item.omath, opts, cancel))
                        .collect()
                };

                for item in work_items {
                    metas.push((item.eq_num, item.target_path));
                }
            }

            if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return Err(Error::Cancelled);
            }

            let mut work_iter = metas.into_iter().zip(prepared_results.drain(..)).peekable();
            let mut pre_iter = collected.into_iter().peekable();

            loop {
                let next_pre_eq = pre_iter.peek().map(|c| match c {
                    CollectedItem::Skipped(_, n) | CollectedItem::Failed(n, _) => *n,
                });
                let next_work_eq = work_iter.peek().map(|((eq_num, _), _)| *eq_num);

                match (next_pre_eq, next_work_eq) {
                    (None, None) => break,
                    (Some(p), Some(w)) if p < w => {
                        if let Some(item) = pre_iter.next() {
                            self.apply_collected_item(xml_path, item);
                        }
                    }
                    (Some(_), None) => {
                        if let Some(item) = pre_iter.next() {
                            self.apply_collected_item(xml_path, item);
                        }
                    }
                    _ => {
                        let Some(((eq_num, target_path), res)) = work_iter.next() else {
                            break;
                        };
                        match res {
                            Ok(PreparedOutcome::Skipped(reason)) => {
                                self.summary.skipped += 1;
                                log::warn!(
                                    "Equation {eq_num}: {reason} is not supported in MathType, skipping"
                                );
                            }
                            Ok(PreparedOutcome::Converted {
                                ole_bin,
                                preview,
                                omath_fallback,
                            }) => {
                                if let Err(e) = self.assemble_converted(
                                    eq_num,
                                    &target_path,
                                    ole_bin,
                                    preview,
                                    omath_fallback.as_deref(),
                                ) {
                                    self.summary.errors += 1;
                                    log::error!("Equation {eq_num} ({xml_path}): {e}");
                                }
                            }
                            Err(Error::Cancelled) => return Err(Error::Cancelled),
                            Err(e) => {
                                self.summary.errors += 1;
                                log::error!("Equation {eq_num} ({xml_path}): {e}");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn apply_collected_item(&mut self, xml_path: &str, item: CollectedItem) {
        match item {
            CollectedItem::Skipped(msg, eq_num) => {
                self.summary.skipped += 1;
                log::warn!("Equation {eq_num}: {msg}");
            }
            CollectedItem::Failed(eq_num, e) => {
                self.summary.errors += 1;
                log::error!("Equation {eq_num} ({xml_path}): {e}");
            }
        }
    }

    fn assemble_converted(
        &mut self,
        eq_num: usize,
        target_path: &[usize],
        ole_bin: Vec<u8>,
        preview: Option<Picture>,
        omath_fallback: Option<&Element>,
    ) -> Result<()> {
        // 5. Store the .bin in the docx embeddings
        let bin_name = format!("word/embeddings/oleObject{}.bin", self.ole_index);
        self.pkg.set(&bin_name, ole_bin);
        let target = format!("embeddings/oleObject{}.bin", self.ole_index);
        self.ole_index += 1;

        // 6. Add relationship
        let rel_id = self.new_rel_id();
        add_relationship(&mut self.rels_root, &rel_id, REL_OLEOBJECT, &target);

        // 7. Add content type for .bin
        ensure_content_type_indexed(
            &mut self.ct_root,
            &mut self.ct_overrides,
            &format!("/{bin_name}"),
            CT_OLEOBJECT,
        );

        // 8. Replace the equation node in the document tree.
        let shape_id = format!("_x0000_i{}", 1024 + eq_num);
        let alt = match preview {
            Some(pv) => {
                let mut img_name = format!("word/media/equation{}.{}", self.img_index, pv.ext);
                while self.pkg.contains(&img_name) {
                    self.img_index += 1;
                    img_name = format!("word/media/equation{}.{}", self.img_index, pv.ext);
                }
                self.img_index += 1;
                self.pkg.set(&img_name, pv.data);
                let img_rid = self.new_img_rel_id();
                add_relationship(
                    &mut self.rels_root,
                    &img_rid,
                    REL_IMAGE,
                    &img_name["word/".len()..],
                );
                ensure_default_extension(&mut self.ct_root, pv.ext, pv.content_type);
                let obj_id = format!(
                    "_{}",
                    1_000_000_000u128 + uuid::Uuid::new_v4().as_u128() % 9_000_000_000u128
                );
                make_object_run(
                    &rel_id,
                    &img_rid,
                    &shape_id,
                    &obj_id,
                    pv.width_pt,
                    pv.height_pt,
                    pv.depth_pt,
                )
            }
            None => {
                let obj_id = format!("{{{}}}", uuid::Uuid::new_v4().to_string().to_uppercase());
                let omath = omath_fallback.ok_or_else(|| {
                    Error::Xml("missing omath fallback for equation without preview".into())
                })?;
                make_ole_xml(&rel_id, &shape_id, &obj_id, 914_400, 457_200, omath)
            }
        };

        let (&slot, holder_path) = target_path
            .split_last()
            .ok_or_else(|| Error::Xml("equation target path is empty".into()))?;
        let holder = element_at_mut(&mut self.doc_root, holder_path)
            .ok_or_else(|| Error::Xml("equation has no parent".into()))?;
        let target_slot = holder
            .children
            .get_mut(slot)
            .ok_or_else(|| Error::Xml("equation target slot out of bounds".into()))?;
        *target_slot = Node::Element(alt);

        self.summary.converted += 1;
        log::info!("Equation {eq_num}: converted → {bin_name}");
        Ok(())
    }
}
