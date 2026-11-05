//! The `.docx` zip package: relationship / content-type helpers and an
//! in-memory entry list that preserves the original order.

use std::collections::{HashMap};
use std::io::{Cursor, Read, Write};


use crate::error::{Result};





use crate::xml::{Element, Node};

use super::consts::*;





pub fn ensure_default_extension(ct_root: &mut Element, ext: &str, content_type: &str) {
    if ct_root.elements().any(|c| {
        c.local == "Default" && c.attr(None, "Extension").map_or(false, |e| e.eq_ignore_ascii_case(ext))
    }) {
        return;
    }
    let mut d = Element::new(Some(NS_CT), ct_root.prefix.as_deref(), "Default");
    d.set_attr(None, None, "Extension", ext);
    d.set_attr(None, None, "ContentType", content_type);
    let pos = ct_root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local == "Override"))
        .unwrap_or(ct_root.children.len());
    ct_root.children.insert(pos, Node::Element(d));
}

// ── Package part helpers ──────────────────────────────────────────────────────

/// Append a `Relationship` to a package-relationships root.
pub fn add_relationship(rels_root: &mut Element, rel_id: &str, rel_type: &str, target: &str) {
    let mut rel = Element::new(Some(NS_PKG_REL), rels_root.prefix.as_deref(), "Relationship");
    rel.set_attr(None, None, "Id", rel_id);
    rel.set_attr(None, None, "Type", rel_type);
    rel.set_attr(None, None, "Target", target);
    rels_root.push(rel);
}

/// Add an `Override` to `[Content_Types].xml` unless `part_name` already has one.
pub fn ensure_content_type(ct_root: &mut Element, part_name: &str, content_type: &str) {
    if ct_root
        .elements()
        .any(|c| c.attr(None, "PartName") == Some(part_name))
    {
        return;
    }
    let mut ov = Element::new(Some(NS_CT), ct_root.prefix.as_deref(), "Override");
    ov.set_attr(None, None, "PartName", part_name);
    ov.set_attr(None, None, "ContentType", content_type);
    ct_root.push(ov);
}

/// One entry of the ZIP container, kept in its original position.
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) data: Vec<u8>,
    pub(super) is_dir: bool,
    pub(super) modified: Option<zip::DateTime>,
}

/// In-memory view of the `.docx` package (original entry order preserved).
pub(super) struct Package {
    pub(super) entries: Vec<Entry>,
    pub(super) index: HashMap<String, usize>,
}

impl Package {
    pub(super) fn read(input: &[u8]) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(Cursor::new(input))?;
        let mut pkg = Package {
            entries: Vec::with_capacity(archive.len()),
            index: HashMap::new(),
        };
        for i in 0..archive.len() {
            let mut f = archive.by_index(i)?;
            let name = f.name().to_owned();
            let is_dir = f.is_dir();
            let modified = Some(f.last_modified());
            let mut data = Vec::new();
            if !is_dir {
                f.read_to_end(&mut data)?;
            }
            match pkg.index.get(&name) {
                // Duplicate names: the last entry wins, like Python's ZipFile.read.
                Some(&idx) => {
                    pkg.entries[idx].data = data;
                    pkg.entries[idx].modified = modified;
                }
                None => {
                    pkg.index.insert(name.clone(), pkg.entries.len());
                    pkg.entries.push(Entry {
                        name,
                        data,
                        is_dir,
                        modified,
                    });
                }
            }
        }
        Ok(pkg)
    }

    pub(super) fn contains(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    pub(super) fn get(&self, name: &str) -> Option<&[u8]> {
        self.index.get(name).map(|&i| self.entries[i].data.as_slice())
    }

    /// Replace the content of `name`, or append it as a new entry.
    pub(super) fn set(&mut self, name: &str, data: Vec<u8>) {
        match self.index.get(name) {
            Some(&i) => self.entries[i].data = data,
            None => {
                self.index.insert(name.to_owned(), self.entries.len());
                self.entries.push(Entry {
                    name: name.to_owned(),
                    data,
                    is_dir: false,
                    modified: None,
                });
            }
        }
    }

    pub(super) fn write(&self) -> Result<Vec<u8>> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for e in &self.entries {
            let mut opts = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            if let Some(m) = e.modified {
                opts = opts.last_modified_time(m);
            }
            if e.is_dir {
                w.add_directory(e.name.as_str(), opts)?;
            } else {
                w.start_file(e.name.as_str(), opts)?;
                w.write_all(&e.data)?;
            }
        }
        Ok(w.finish()?.into_inner())
    }
}
