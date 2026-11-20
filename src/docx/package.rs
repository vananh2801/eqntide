//! The `.docx` zip package: relationship / content-type helpers and an
//! in-memory entry list that preserves the original order.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use crate::error::{Error, Result};
use crate::xml::{Element, Node};

use super::consts::*;

const MAX_DOCX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DOCX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

pub fn ensure_default_extension(ct_root: &mut Element, ext: &str, content_type: &str) {
    if ct_root.elements().any(|c| {
        c.local == "Default"
            && c.attr(None, "Extension")
                .is_some_and(|e| e.eq_ignore_ascii_case(ext))
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
    let mut rel = Element::new(
        Some(NS_PKG_REL),
        rels_root.prefix.as_deref(),
        "Relationship",
    );
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
        let mut total_uncompressed: u64 = 0;
        for i in 0..archive.len() {
            let f = archive.by_index(i)?;
            let name = f.name().to_owned();
            let is_dir = f.is_dir();
            let modified = Some(f.last_modified());
            let mut data = Vec::new();
            if !is_dir {
                if f.size() > MAX_DOCX_ENTRY_BYTES {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("ZIP entry '{name}' exceeds maximum uncompressed size limit"),
                    )));
                }
                f.take(MAX_DOCX_ENTRY_BYTES + 1).read_to_end(&mut data)?;
                if data.len() as u64 > MAX_DOCX_ENTRY_BYTES {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("ZIP entry '{name}' exceeds maximum uncompressed size limit"),
                    )));
                }
                total_uncompressed = total_uncompressed.saturating_add(data.len() as u64);
                if total_uncompressed > MAX_DOCX_TOTAL_BYTES {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "DOCX archive exceeds maximum total decompressed size limit",
                    )));
                }
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
        self.index
            .get(name)
            .map(|&i| self.entries[i].data.as_slice())
    }

    pub(super) fn part_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.name.clone())
            .collect()
    }

    pub(super) fn remove(&mut self, name: &str) -> bool {
        if self.index.remove(name).is_some() {
            self.entries.retain(|e| e.name != name);
            self.index.clear();
            for (idx, e) in self.entries.iter().enumerate() {
                self.index.insert(e.name.clone(), idx);
            }
            true
        } else {
            false
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_compressed_docx_entry_is_rejected() {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        w.start_file("word/document.xml", opts).unwrap();
        let chunk = vec![b'A'; 1024 * 1024];
        for _ in 0..65 {
            w.write_all(&chunk).unwrap();
        }
        let zip_bytes = w.finish().unwrap().into_inner();
        let err = match Package::read(&zip_bytes) {
            Ok(_) => panic!("65 MiB entry must be rejected"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("size limit"));
    }
}
