//! The `.docx` zip package: relationship / content-type helpers and an
//! in-memory entry list that preserves the original order.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::xml::{self, Element, Node};

use super::consts::*;

const MAX_DOCX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DOCX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const STREAM_COMPRESS_BATCH: usize = 64;

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

/// Collect existing `Override` `PartName`s from `[Content_Types].xml` for O(1) membership checks.
pub(super) fn collect_override_part_names(ct_root: &Element) -> HashSet<String> {
    ct_root
        .elements()
        .filter(|c| c.local == "Override")
        .filter_map(|c| c.attr(None, "PartName").map(str::to_owned))
        .collect()
}

/// O(1) variant of [`ensure_content_type`] backed by a cached set of override `PartName`s.
pub(super) fn ensure_content_type_indexed(
    ct_root: &mut Element,
    known_overrides: &mut HashSet<String>,
    part_name: &str,
    content_type: &str,
) {
    if !known_overrides.insert(part_name.to_owned()) {
        return;
    }
    let mut ov = Element::new(Some(NS_CT), ct_root.prefix.as_deref(), "Override");
    ov.set_attr(None, None, "PartName", part_name);
    ov.set_attr(None, None, "ContentType", content_type);
    ct_root.push(ov);
}

/// Storage backing for one ZIP entry.
pub(super) enum EntryData {
    Loaded(Vec<u8>),
    Deferred { archive_index: usize },
    Removed,
}

/// One entry of the ZIP container, kept in its original position.
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) data: EntryData,
    pub(super) is_dir: bool,
    pub(super) modified: Option<zip::DateTime>,
}

/// Read-only view combining [`Package`] with a transient per-chunk cache of
/// lazily decompressed entries for lock-free `rayon` reads.
pub(super) struct PartLookup<'a> {
    pub(super) pkg: &'a Package,
    pub(super) cache: &'a HashMap<String, Vec<u8>>,
}

impl<'a> PartLookup<'a> {
    pub(super) fn get(&self, name: &str) -> Option<&[u8]> {
        self.cache
            .get(name)
            .map(Vec::as_slice)
            .or_else(|| self.pkg.get(name))
    }
}

struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn read_bounded_zip_entry<R: Read>(f: R, name: &str) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    f.take(MAX_DOCX_ENTRY_BYTES + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_DOCX_ENTRY_BYTES {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("ZIP entry '{name}' exceeds maximum uncompressed size limit"),
        )));
    }
    Ok(data)
}

/// In-memory or file-backed lazy view of the `.docx` package (original entry order preserved).
pub(super) struct Package {
    pub(super) entries: Vec<Entry>,
    pub(super) index: HashMap<String, usize>,
    source: Option<zip::ZipArchive<BufReader<File>>>,
}

impl Package {
    pub(super) fn read(input: &[u8]) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(Cursor::new(input))?;
        let mut pkg = Package {
            entries: Vec::with_capacity(archive.len()),
            index: HashMap::new(),
            source: None,
        };
        let mut total_uncompressed: u64 = 0;
        for i in 0..archive.len() {
            let f = archive.by_index(i)?;
            let name = f.name()?.into_owned();
            let is_dir = f.is_dir();
            let modified = f.last_modified();
            let mut data = Vec::new();
            if !is_dir {
                if f.size() > MAX_DOCX_ENTRY_BYTES {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("ZIP entry '{name}' exceeds maximum uncompressed size limit"),
                    )));
                }
                data = read_bounded_zip_entry(f, &name)?;
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
                    pkg.entries[idx].data = EntryData::Loaded(data);
                    pkg.entries[idx].modified = modified;
                }
                None => {
                    pkg.index.insert(name.clone(), pkg.entries.len());
                    pkg.entries.push(Entry {
                        name,
                        data: EntryData::Loaded(data),
                        is_dir,
                        modified,
                    });
                }
            }
        }
        Ok(pkg)
    }

    /// Open a `.docx` archive on disk lazily, scanning only central-directory
    /// metadata and deferring entry decompression until [`load_xml`](Self::load_xml) or
    /// [`load_transient_batch`](Self::load_transient_batch).
    pub(super) fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(BufReader::new(file))?;
        let mut pkg = Package {
            entries: Vec::with_capacity(archive.len()),
            index: HashMap::new(),
            source: None,
        };
        let mut total_declared: u64 = 0;
        for i in 0..archive.len() {
            let f = archive.by_index_raw(i)?;
            let name = f.name()?.into_owned();
            let is_dir = f.is_dir();
            let modified = f.last_modified();
            let size = f.size();
            if !is_dir {
                if size > MAX_DOCX_ENTRY_BYTES {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("ZIP entry '{name}' exceeds maximum uncompressed size limit"),
                    )));
                }
                total_declared = total_declared.saturating_add(size);
                if total_declared > MAX_DOCX_TOTAL_BYTES {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "DOCX archive exceeds maximum total decompressed size limit",
                    )));
                }
            }
            let entry_data = EntryData::Deferred { archive_index: i };
            match pkg.index.get(&name) {
                Some(&idx) => {
                    pkg.entries[idx].data = entry_data;
                    pkg.entries[idx].is_dir = is_dir;
                    pkg.entries[idx].modified = modified;
                }
                None => {
                    pkg.index.insert(name.clone(), pkg.entries.len());
                    pkg.entries.push(Entry {
                        name,
                        data: entry_data,
                        is_dir,
                        modified,
                    });
                }
            }
        }
        pkg.source = Some(archive);
        Ok(pkg)
    }

    pub(super) fn contains(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    pub(super) fn get(&self, name: &str) -> Option<&[u8]> {
        let &idx = self.index.get(name)?;
        match &self.entries[idx].data {
            EntryData::Loaded(data) => Some(data.as_slice()),
            EntryData::Deferred { .. } | EntryData::Removed => None,
        }
    }

    // Read one original entry without changing its backing. Validating the
    // actual size against the size counted by open() keeps its total valid.
    fn read_deferred(&mut self, archive_index: usize) -> Result<Vec<u8>> {
        let archive = self.source.as_mut().ok_or_else(|| {
            Error::Io(std::io::Error::other(
                "missing source archive for deferred entry",
            ))
        })?;
        let file = archive.by_index(archive_index)?;
        let size = file.size();
        if size > MAX_DOCX_ENTRY_BYTES {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ZIP entry exceeds maximum uncompressed size limit",
            )));
        }
        let mut bytes = Vec::new();
        file.take(size + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ZIP entry size differs from its directory record",
            )));
        }
        Ok(bytes)
    }

    /// Parse a part without retaining decompressed input or marking it modified.
    pub(super) fn load_xml(&mut self, name: &str) -> Result<Option<Element>> {
        let Some(&idx) = self.index.get(name) else {
            return Ok(None);
        };
        match &self.entries[idx].data {
            EntryData::Loaded(data) => xml::parse(data).map(Some),
            EntryData::Deferred { archive_index } => {
                xml::parse(&self.read_deferred(*archive_index)?).map(Some)
            }
            EntryData::Removed => Ok(None),
        }
    }

    /// Decompress a transient batch of deferred entries into a caller-owned map
    /// without caching them permanently inside `self.entries`.
    pub(super) fn load_transient_batch<'a>(
        &mut self,
        names: impl IntoIterator<Item = &'a str>,
    ) -> Result<HashMap<String, Vec<u8>>> {
        let mut out = HashMap::new();
        for name in names {
            if out.contains_key(name) {
                continue;
            }
            let Some(&idx) = self.index.get(name) else {
                continue;
            };
            if let EntryData::Deferred { archive_index, .. } = self.entries[idx].data {
                let data = self.read_deferred(archive_index)?;
                out.insert(name.to_owned(), data);
            }
        }
        Ok(out)
    }

    pub(super) fn part_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| !e.is_dir && !matches!(e.data, EntryData::Removed))
            .map(|e| e.name.clone())
            .collect()
    }

    pub(super) fn remove(&mut self, name: &str) -> bool {
        if let Some(idx) = self.index.remove(name) {
            self.entries[idx].data = EntryData::Removed;
            true
        } else {
            false
        }
    }

    /// Replace the content of `name`, or append it as a new entry.
    pub(super) fn set(&mut self, name: &str, data: Vec<u8>) {
        match self.index.get(name) {
            Some(&i) => {
                self.entries[i].data = EntryData::Loaded(data);
            }
            None => {
                self.index.insert(name.to_owned(), self.entries.len());
                self.entries.push(Entry {
                    name: name.to_owned(),
                    data: EntryData::Loaded(data),
                    is_dir: false,
                    modified: None,
                });
            }
        }
    }

    pub(super) fn write(&self) -> Result<Vec<u8>> {
        let loaded_entries: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|e| !matches!(e.data, EntryData::Removed))
            .collect();
        if loaded_entries.len() < 16 || rayon::current_num_threads() <= 1 {
            return write_entries_zip_refs(&loaded_entries);
        }

        let chunk_size =
            (loaded_entries.len() / (rayon::current_num_threads().max(1) * 2)).clamp(8, 64);
        let chunks: Vec<Result<Vec<u8>>> = loaded_entries
            .par_chunks(chunk_size)
            .map(write_entries_zip_refs)
            .collect();

        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for chunk_res in chunks {
            let chunk_bytes = chunk_res?;
            let mut archive = zip::ZipArchive::new(Cursor::new(chunk_bytes))?;
            for i in 0..archive.len() {
                let file = archive.by_index(i)?;
                w.raw_copy_file(file)?;
            }
        }
        Ok(w.finish()?.into_inner())
    }

    /// Stream the package directly to `output_path` via a sibling temporary file,
    /// passing untouched entries through without decompression and compressing
    /// modified/new entries in bounded parallel batches.
    pub(super) fn write_to_file_atomic(
        &mut self,
        output_path: &Path,
        cancel: Option<&AtomicBool>,
    ) -> Result<()> {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        // rename replaces a symlink itself. Follow the final link as a normal
        // file write would, including relative links to a new destination.
        let mut destination = output_path.to_owned();
        let mut followed_links = HashSet::new();
        let permissions = loop {
            match std::fs::symlink_metadata(&destination) {
                Ok(meta) if meta.is_symlink() => {
                    if !followed_links.insert(destination.clone()) || followed_links.len() > 40 {
                        return Err(Error::Io(std::io::Error::other("too many output symlinks")));
                    }
                    let target = std::fs::read_link(&destination)?;
                    destination.pop();
                    destination.push(target);
                }
                Ok(meta) if meta.is_file() => {
                    // Respect existing write restrictions before replacing the inode.
                    OpenOptions::new().write(true).open(&destination)?;
                    break Some(meta.permissions());
                }
                Ok(_) => {
                    return Err(Error::Io(std::io::Error::other(
                        "expected a regular output file",
                    )))
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => break None,
                Err(err) => return Err(err.into()),
            }
        };
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;

        let temp_path = parent.join(format!(".eqntide-tmp-{}.docx", uuid::Uuid::new_v4()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let temp_file = options.open(&temp_path)?;
        let mut guard = TempFileGuard::new(temp_path.clone());

        {
            let mut w = zip::ZipWriter::new(BufWriter::new(temp_file));
            let mut loaded_batch: Vec<usize> = Vec::with_capacity(STREAM_COMPRESS_BATCH);

            for idx in 0..self.entries.len() {
                if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                    return Err(Error::Cancelled);
                }
                match &self.entries[idx].data {
                    EntryData::Removed => continue,
                    EntryData::Loaded(_) if !self.entries[idx].is_dir => {
                        loaded_batch.push(idx);
                        if loaded_batch.len() >= STREAM_COMPRESS_BATCH {
                            flush_loaded_batch(&self.entries, &mut loaded_batch, &mut w)?;
                        }
                    }
                    EntryData::Loaded(_) | EntryData::Deferred { .. } => {
                        if !loaded_batch.is_empty() {
                            flush_loaded_batch(&self.entries, &mut loaded_batch, &mut w)?;
                        }
                        let e = &self.entries[idx];
                        if e.is_dir {
                            let mut opts = zip::write::SimpleFileOptions::default()
                                .compression_method(zip::CompressionMethod::Deflated);
                            if let Some(m) = e.modified {
                                opts = opts.last_modified_time(m);
                            }
                            w.add_directory(e.name.as_str(), opts)?;
                        } else if let EntryData::Deferred { archive_index, .. } = e.data {
                            let archive = self.source.as_mut().ok_or_else(|| {
                                Error::Io(std::io::Error::new(
                                    std::io::ErrorKind::InvalidInput,
                                    "missing source archive for deferred entry",
                                ))
                            })?;
                            let file = archive.by_index_raw(archive_index)?;
                            w.raw_copy_file(file)?;
                        }
                    }
                }
            }
            if !loaded_batch.is_empty() {
                flush_loaded_batch(&self.entries, &mut loaded_batch, &mut w)?;
            }
            if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return Err(Error::Cancelled);
            }
            let buf_writer = w.finish()?;
            let file = buf_writer
                .into_inner()
                .map_err(|e| Error::Io(e.into_error()))?;
            if let Some(permissions) = permissions {
                file.set_permissions(permissions)?;
            }
            file.sync_all()?;
        }

        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        // Close the source archive handle before replacing `output_path` (required on Windows for in-place overwrite).
        self.source = None;
        std::fs::rename(&temp_path, &destination)?;
        guard.disarm();
        Ok(())
    }
}

fn flush_loaded_batch<W: Write + std::io::Seek>(
    entries: &[Entry],
    batch_indices: &mut Vec<usize>,
    writer: &mut zip::ZipWriter<W>,
) -> Result<()> {
    let refs: Vec<&Entry> = batch_indices.iter().map(|&i| &entries[i]).collect();
    if refs.len() < 16 || rayon::current_num_threads() <= 1 {
        write_entries_into_writer(&refs, writer)?;
    } else {
        let chunk_size = (refs.len() / rayon::current_num_threads().max(1)).clamp(4, 16);
        let chunks: Vec<Result<Vec<u8>>> = refs
            .par_chunks(chunk_size)
            .map(write_entries_zip_refs)
            .collect();
        for chunk_res in chunks {
            let chunk_bytes = chunk_res?;
            let mut archive = zip::ZipArchive::new(Cursor::new(chunk_bytes))?;
            for i in 0..archive.len() {
                let file = archive.by_index(i)?;
                writer.raw_copy_file(file)?;
            }
        }
    }
    batch_indices.clear();
    Ok(())
}

fn write_entries_into_writer<W: Write + std::io::Seek>(
    entries: &[&Entry],
    w: &mut zip::ZipWriter<W>,
) -> Result<()> {
    for &e in entries {
        let mut opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        if let Some(m) = e.modified {
            opts = opts.last_modified_time(m);
        }
        if e.is_dir {
            w.add_directory(e.name.as_str(), opts)?;
        } else {
            let EntryData::Loaded(ref data) = e.data else {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("cannot compress non-loaded entry '{}'", e.name),
                )));
            };
            w.start_file(e.name.as_str(), opts)?;
            w.write_all(data)?;
        }
    }
    Ok(())
}

fn write_entries_zip_refs(entries: &[&Entry]) -> Result<Vec<u8>> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    write_entries_into_writer(entries, &mut w)?;
    Ok(w.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_compressed_docx_entry_is_rejected() {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
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

    #[test]
    fn parallel_package_write_preserves_order_dirs_and_payloads() {
        let mut entries = Vec::new();
        entries.push(Entry {
            name: "word/".into(),
            data: EntryData::Loaded(Vec::new()),
            is_dir: true,
            modified: None,
        });
        for i in 0..40 {
            entries.push(Entry {
                name: format!("word/part_{i:02}.bin"),
                data: EntryData::Loaded(vec![(i % 251) as u8; 512 + i * 17]),
                is_dir: false,
                modified: None,
            });
        }
        let pkg = Package {
            index: entries
                .iter()
                .enumerate()
                .map(|(i, e)| (e.name.clone(), i))
                .collect(),
            entries,
            source: None,
        };

        let bytes = pkg.write().unwrap();
        let archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(archive.len(), 41);
        let roundtrip = Package::read(&bytes).unwrap();
        assert_eq!(roundtrip.entries.len(), 41);
        assert!(roundtrip.entries[0].is_dir);
        assert_eq!(roundtrip.entries[0].name, "word/");
        for i in 0..40 {
            let name = format!("word/part_{i:02}.bin");
            assert_eq!(roundtrip.entries[i + 1].name, name);
            assert_eq!(
                roundtrip.get(&name).unwrap(),
                vec![(i % 251) as u8; 512 + i * 17].as_slice()
            );
        }
    }

    #[test]
    fn lazy_open_defers_untouched_entries_and_rejects_oversized_archive() {
        let dir = std::env::temp_dir().join(format!("eqntide_pkg_lazy_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("sample.docx");
        {
            let mut w = zip::ZipWriter::new(File::create(&path).unwrap());
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            w.add_directory("word/", opts).unwrap();
            w.start_file("word/document.xml", opts).unwrap();
            w.write_all(b"<w:document/>").unwrap();
            w.start_file("word/media/keep.bin", opts).unwrap();
            w.write_all(b"untouched-media-payload").unwrap();
            w.finish().unwrap();
        }

        let mut pkg = Package::open(&path).unwrap();
        // Deferred entries are not decompressed until explicitly loaded.
        assert!(pkg.get("word/document.xml").is_none());
        assert!(pkg.get("word/media/keep.bin").is_none());
        assert_eq!(
            pkg.load_xml("word/document.xml").unwrap().unwrap().local,
            "document"
        );
        // Inspecting the XML leaves both original entries deferred.
        assert!(pkg.get("word/document.xml").is_none());
        assert!(pkg.get("word/media/keep.bin").is_none());

        let out_path = dir.join("out.docx");
        pkg.set("word/document.xml", b"<w:document updated=\"1\"/>".to_vec());
        for i in 0..70 {
            pkg.set(
                &format!("word/embeddings/oleObject{i}.bin"),
                vec![i as u8; 64],
            );
        }
        pkg.write_to_file_atomic(&out_path, None).unwrap();

        let roundtrip_bytes = std::fs::read(&out_path).unwrap();
        let roundtrip = Package::read(&roundtrip_bytes).unwrap();
        assert_eq!(
            roundtrip.get("word/document.xml").unwrap(),
            b"<w:document updated=\"1\"/>"
        );
        assert_eq!(
            roundtrip.get("word/media/keep.bin").unwrap(),
            b"untouched-media-payload"
        );
        assert_eq!(
            roundtrip.get("word/embeddings/oleObject69.bin").unwrap(),
            &[69u8; 64]
        );

        // Oversized entry rejection on Package::open
        let bad_path = dir.join("oversized.docx");
        {
            let mut w = zip::ZipWriter::new(File::create(&bad_path).unwrap());
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            w.start_file("word/document.xml", opts).unwrap();
            let chunk = vec![b'B'; 1024 * 1024];
            for _ in 0..65 {
                w.write_all(&chunk).unwrap();
            }
            w.finish().unwrap();
        }
        let err = match Package::open(&bad_path) {
            Ok(_) => panic!("65 MiB entry must be rejected"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("size limit"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_private_mode_and_relative_symlink() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = std::env::temp_dir().join(format!("eqntide-link-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        let path = dir.join("docs/private.docx");
        zip::ZipWriter::new(File::create(&path).unwrap())
            .finish()
            .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = dir.join("alias.docx");
        symlink("docs/private.docx", &link).unwrap();
        let mut pkg = Package::open(&link).unwrap();
        pkg.set("word/document.xml", b"<document/>".to_vec());
        pkg.write_to_file_atomic(&link, None).unwrap();
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            Path::new("docs/private.docx")
        );
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(Package::open(&path).unwrap().contains("word/document.xml"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn write_to_file_atomic_supports_inplace_and_cancel_leaves_no_temp_files() {
        let dir = std::env::temp_dir().join(format!("eqntide_pkg_atomic_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("inplace.docx");
        {
            let mut w = zip::ZipWriter::new(File::create(&path).unwrap());
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("word/document.xml", opts).unwrap();
            w.write_all(b"<w:document>old</w:document>").unwrap();
            w.start_file("word/media/img.bin", opts).unwrap();
            w.write_all(b"keep").unwrap();
            w.finish().unwrap();
        }

        // Cancelled write preserves original file and leaves zero temp files.
        let mut pkg_cancel = Package::open(&path).unwrap();
        pkg_cancel.set(
            "word/document.xml",
            b"<w:document>cancelled</w:document>".to_vec(),
        );
        let stop = AtomicBool::new(true);
        let err = pkg_cancel
            .write_to_file_atomic(&path, Some(&stop))
            .unwrap_err();
        assert!(matches!(err, Error::Cancelled));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let unchanged = Package::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            unchanged.get("word/document.xml").unwrap(),
            b"<w:document>old</w:document>"
        );

        // Successful in-place write replaces destination cleanly without temp files.
        let mut pkg_ok = Package::open(&path).unwrap();
        pkg_ok.set(
            "word/document.xml",
            b"<w:document>new</w:document>".to_vec(),
        );
        pkg_ok.write_to_file_atomic(&path, None).unwrap();
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let updated = Package::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            updated.get("word/document.xml").unwrap(),
            b"<w:document>new</w:document>"
        );
        assert_eq!(updated.get("word/media/img.bin").unwrap(), b"keep");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
