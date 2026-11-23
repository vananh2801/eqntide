//! Minimal OLE Compound File Binary (CFB v3, 512-byte sectors) writer and reader,
//! plus WMF MathType comment extractor.
//!
//! Produces the same container layout MathType itself writes for an equation:
//! four streams (`\x01Ole`, `\x01CompObj`, `\x03ObjInfo`, `Equation Native`),
//! small streams stored in the mini-stream, and the MathType root CLSID
//! `{0002CE03-0000-0000-C000-000000000046}`. The three auxiliary streams are
//! byte-for-byte copies from a genuine MathType 5 file.
//!
//! Also provides [`extract_mtef_from_ole`] and [`extract_mtef_from_wmf`] for the
//! reverse pipeline (MathType OLE / WMF → MTEF → MathML → OMML).
//!
//! Reference: \[MS-CFB\] <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb>

use crate::error::{Error, Result};

/// CFB signature.
pub const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
/// Sector size in bytes.
pub const SECTOR: usize = 512;
/// Mini-sector size in bytes.
pub const MINI_SECTOR: usize = 64;
/// Unused FAT entry / "no stream" directory id.
pub const FREESECT: u32 = 0xFFFF_FFFF;
/// End of a sector chain.
pub const ENDOFCHAIN: u32 = 0xFFFF_FFFE;
/// Sector holding the FAT itself.
pub const FATSECT: u32 = 0xFFFF_FFFD;
/// Sector holding a DIFAT sector.
pub const DIFSECT: u32 = 0xFFFF_FFFC;
/// Streams smaller than this live in the mini-stream.
pub const MINI_CUTOFF: usize = 4096;
/// [MS-CFB] v3 header size, which is also one regular sector.
const CFB_V3_HEADER_SIZE: usize = 512;
/// [MS-CFB] v3 directory-entry size.
const CFB_DIRECTORY_ENTRY_SIZE: usize = 128;
/// [MS-CFB] header DIFAT capacity before DIFAT sectors are required.
const CFB_HEADER_DIFAT_ENTRIES: usize = 109;
/// [MS-CFB] v3 sector-shift value for 512-byte regular sectors.
const CFB_V3_SECTOR_SHIFT: u16 = 9;
/// [MS-CFB] mini-sector-shift value for 64-byte mini sectors.
const CFB_MINI_SECTOR_SHIFT: u16 = 6;
/// ProgID MathType registers for its equation objects.
pub const PROG_ID: &str = "Equation.DSMT4";

/// Known ProgIDs for MathType and Equation Editor OLE objects.
pub const MATHTYPE_PROG_IDS: &[&str] = &["Equation.DSMT4", "Equation.3", "Equation.2"];

/// Known stream names in MathType OLE containers.
pub const MTEF_STREAM_NAMES: &[&str] = &["Equation Native", "EquationNative", "Equation"];

const COMPOBJ: &[u8] = include_bytes!("../assets/compobj.bin");
const OLE_STREAM: &[u8] = include_bytes!("../assets/ole.bin");
const OBJINFO: &[u8] = include_bytes!("../assets/objinfo.bin");

/// `{0002CE03-0000-0000-C000-000000000046}` in on-disk byte order.
const MATHTYPE_CLSID: [u8; 16] = [
    0x03, 0xCE, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

fn put_u16(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

fn get_u16(buf: &[u8], off: usize) -> Option<u16> {
    let b = buf.get(off..off + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

fn get_u32(buf: &[u8], off: usize) -> Option<u32> {
    let b = buf.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn div_ceil(a: usize, b: usize) -> usize {
    a.div_ceil(b)
}

/// Build one 128-byte directory entry.
///
/// `entry_type`: 0 unused, 1 storage, 2 stream, 5 root. `color`: 0 red, 1 black.
#[allow(clippy::too_many_arguments)]
pub fn dir_entry(
    name: &str,
    entry_type: u8,
    color: u8,
    child: u32,
    left: u32,
    right: u32,
    start_sect: u32,
    size: u32,
    clsid: [u8; 16],
) -> [u8; 128] {
    let mut name_utf16: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
    name_utf16.extend_from_slice(&[0, 0]);
    let name_len = name_utf16.len();

    let mut entry = [0u8; 128];
    entry[..name_utf16.len()].copy_from_slice(&name_utf16);
    put_u16(&mut entry, 64, name_len as u16);
    entry[66] = entry_type;
    entry[67] = color;
    put_u32(&mut entry, 68, left);
    put_u32(&mut entry, 72, right);
    put_u32(&mut entry, 76, child);
    entry[80..96].copy_from_slice(&clsid);
    put_u32(&mut entry, 116, start_sect);
    put_u32(&mut entry, 120, size);
    entry
}

fn unused_entry() -> [u8; 128] {
    let mut e = [0u8; 128];
    put_u32(&mut e, 68, FREESECT);
    put_u32(&mut e, 72, FREESECT);
    put_u32(&mut e, 76, FREESECT);
    e
}

/// Where a stream's data ended up.
struct Placed {
    start: u32,
    size: usize,
}

/// Return the bytes of an OLE compound file holding a MathType equation whose
/// `Equation Native` stream is `equation_native`.
///
/// # Errors
///
/// [`Error::EquationTooLarge`] when the stream would need more than the 109
/// FAT sectors that fit in the CFB header (about 7 MB).
pub fn make_ole(equation_native: &[u8]) -> Result<Vec<u8>> {
    write_mathtype_cfb(equation_native)
}

/// Write the fixed-layout CFB v3 container used by MathType equation objects.
fn write_mathtype_cfb(equation_native: &[u8]) -> Result<Vec<u8>> {
    // Directory order mirrors MathType: index 1..=4 are the root's children.
    let streams: [(&str, &[u8]); 4] = [
        ("\u{1}Ole", OLE_STREAM),
        ("\u{1}CompObj", COMPOBJ),
        ("\u{3}ObjInfo", OBJINFO),
        ("Equation Native", equation_native),
    ];

    // ── Split streams into mini-stream residents and regular-sector ones ────
    let mut mini_stream: Vec<u8> = Vec::new();
    let mut mini_fat: Vec<u32> = Vec::new();
    let mut big_sizes: Vec<usize> = Vec::new(); // sectors per big stream, in order
    let mut placed: Vec<Placed> = Vec::new();
    for (_, data) in &streams {
        if data.len() < MINI_CUTOFF {
            let first = mini_fat.len();
            let n = div_ceil(data.len().max(1), MINI_SECTOR);
            for i in 0..n {
                mini_fat.push(if i + 1 < n {
                    (first + i + 1) as u32
                } else {
                    ENDOFCHAIN
                });
            }
            mini_stream.extend_from_slice(data);
            mini_stream.resize((first + n) * MINI_SECTOR, 0);
            placed.push(Placed {
                start: first as u32,
                size: data.len(),
            });
        } else {
            big_sizes.push(div_ceil(data.len(), SECTOR));
            placed.push(Placed {
                start: 0,
                size: data.len(),
            }); // fixed up below
        }
    }

    // ── Sector budget ───────────────────────────────────────────────────────
    let n_dir = 2; // 5 entries → 2 sectors of 4
    let n_minifat = div_ceil(mini_fat.len(), SECTOR / 4);
    let n_ministream = div_ceil(mini_stream.len(), SECTOR);
    let n_big: usize = big_sizes.iter().sum();
    let others = n_dir + n_minifat + n_ministream + n_big;
    let mut n_fat = 1;
    while n_fat * (SECTOR / 4) < n_fat + others {
        n_fat += 1;
    }
    if n_fat > CFB_HEADER_DIFAT_ENTRIES {
        return Err(Error::EquationTooLarge {
            stream_len: equation_native.len(),
        });
    }

    let dir_first = n_fat;
    let minifat_first = dir_first + n_dir;
    let ministream_first = minifat_first + n_minifat;
    let big_first = ministream_first + n_ministream;
    let total = big_first + n_big;

    // ── FAT ─────────────────────────────────────────────────────────────────
    let mut fat = vec![FREESECT; n_fat * (SECTOR / 4)];
    for f in fat.iter_mut().take(n_fat) {
        *f = FATSECT;
    }
    let chain = |fat: &mut [u32], first: usize, count: usize| {
        for i in 0..count {
            fat[first + i] = if i + 1 < count {
                (first + i + 1) as u32
            } else {
                ENDOFCHAIN
            };
        }
    };
    chain(&mut fat, dir_first, n_dir);
    chain(&mut fat, minifat_first, n_minifat);
    chain(&mut fat, ministream_first, n_ministream);
    let mut cursor = big_first;
    let mut big_iter = big_sizes.iter();
    for (i, (_, data)) in streams.iter().enumerate() {
        if data.len() >= MINI_CUTOFF {
            let count = *big_iter
                .next()
                .ok_or_else(|| Error::InvalidOle("big stream size count mismatch".into()))?;
            chain(&mut fat, cursor, count);
            placed[i].start = cursor as u32;
            cursor += count;
        }
    }

    // ── Header ──────────────────────────────────────────────────────────────
    let mut header = [0u8; CFB_V3_HEADER_SIZE];
    header[0..8].copy_from_slice(&MAGIC);
    put_u16(&mut header, 24, 0x003E); // minor version
    put_u16(&mut header, 26, 0x0003); // major version (v3)
    put_u16(&mut header, 28, 0xFFFE); // little-endian
    put_u16(&mut header, 30, CFB_V3_SECTOR_SHIFT);
    put_u16(&mut header, 32, CFB_MINI_SECTOR_SHIFT);
    put_u32(&mut header, 44, n_fat as u32); // NumFATSectors
    put_u32(&mut header, 48, dir_first as u32); // FirstDirectorySector
    put_u32(&mut header, 56, MINI_CUTOFF as u32);
    put_u32(
        &mut header,
        60,
        if n_minifat > 0 {
            minifat_first as u32
        } else {
            ENDOFCHAIN
        },
    );
    put_u32(&mut header, 64, n_minifat as u32);
    put_u32(&mut header, 68, ENDOFCHAIN); // no DIFAT sectors
    put_u32(&mut header, 72, 0);
    for i in 0..CFB_HEADER_DIFAT_ENTRIES {
        put_u32(
            &mut header,
            76 + i * 4,
            if i < n_fat { i as u32 } else { FREESECT },
        );
    }

    // ── Directory (balanced tree exactly like MathType writes it) ───────────
    //        ObjInfo(3)
    //       /          \
    //  CompObj(2)   Equation Native(4)
    //     /
    //  Ole(1)
    let root = dir_entry(
        "Root Entry",
        5,
        0,
        3,
        FREESECT,
        FREESECT,
        if n_ministream > 0 {
            ministream_first as u32
        } else {
            ENDOFCHAIN
        },
        mini_stream.len() as u32,
        MATHTYPE_CLSID,
    );
    let entry = |idx: usize, left: u32, right: u32| {
        let color = if idx == 0 { 0 } else { 1 };
        dir_entry(
            streams[idx].0,
            2,
            color,
            FREESECT,
            left,
            right,
            placed[idx].start,
            placed[idx].size as u32,
            [0; 16],
        )
    };
    let mut dir = Vec::with_capacity(n_dir * SECTOR);
    dir.extend_from_slice(&root);
    dir.extend_from_slice(&entry(0, FREESECT, FREESECT)); // 1: Ole
    dir.extend_from_slice(&entry(1, 1, FREESECT)); //          2: CompObj (left = Ole)
    dir.extend_from_slice(&entry(2, 2, 4)); //                 3: ObjInfo (left, right)
    dir.extend_from_slice(&entry(3, FREESECT, FREESECT)); //   4: Equation Native
    while dir.len() < n_dir * SECTOR {
        dir.extend_from_slice(&unused_entry());
    }

    // ── Assemble ────────────────────────────────────────────────────────────
    let mut out = Vec::with_capacity((total + 1) * SECTOR);
    out.extend_from_slice(&header);
    for v in &fat[..n_fat * (SECTOR / 4)] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&dir);
    let mut mf: Vec<u8> = mini_fat.iter().flat_map(|v| v.to_le_bytes()).collect();
    mf.resize(n_minifat * SECTOR, 0xFF);
    out.extend_from_slice(&mf);
    let mut ms = mini_stream;
    ms.resize(n_ministream * SECTOR, 0);
    out.extend_from_slice(&ms);
    for (_, data) in &streams {
        if data.len() >= MINI_CUTOFF {
            out.extend_from_slice(data);
            out.resize(out.len() + (SECTOR - data.len() % SECTOR) % SECTOR, 0);
        }
    }
    debug_assert_eq!(out.len(), (total + 1) * SECTOR);
    Ok(out)
}

// ── OLE Compound File Reader & MTEF Extractor ─────────────────────────────────

struct DirItem {
    name: String,
    entry_type: u8,
    start_sect: u32,
    size: usize,
}

/// Parsed CFB header fields needed to locate the allocation tables and streams.
struct CfbHeader {
    sector_size: usize,
    mini_sector_size: usize,
    first_dir_sect: u32,
    mini_cutoff: usize,
    first_minifat_sect: u32,
    num_minifat_sects: usize,
    first_difat_sect: u32,
    num_difat_sects: usize,
}

const MAX_CFB_MINI_CUTOFF: usize = 65_536;

impl CfbHeader {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < SECTOR || data.get(0..8)? != MAGIC {
            return None;
        }

        let sector_shift = get_u16(data, 30)? as usize;
        let mini_shift = get_u16(data, 32)? as usize;
        if !(7..=16).contains(&sector_shift) || !(4..=sector_shift).contains(&mini_shift) {
            return None;
        }
        let sector_size = 1usize << sector_shift;
        let mini_cutoff = get_u32(data, 56)? as usize;
        if mini_cutoff > MAX_CFB_MINI_CUTOFF {
            return None;
        }

        Some(Self {
            sector_size,
            mini_sector_size: 1usize << mini_shift,
            first_dir_sect: get_u32(data, 48)?,
            mini_cutoff,
            first_minifat_sect: get_u32(data, 60)?,
            num_minifat_sects: get_u32(data, 64)? as usize,
            first_difat_sect: get_u32(data, 68)?,
            num_difat_sects: get_u32(data, 72)? as usize,
        })
    }
}

fn read_sector(data: &[u8], sector_size: usize, sect: u32) -> Option<&[u8]> {
    let off = (sect as usize).checked_add(1)?.checked_mul(sector_size)?;
    data.get(off..off.checked_add(sector_size)?)
}

fn read_chain(
    data: &[u8],
    sector_size: usize,
    fat: &[u32],
    start_sect: u32,
    max_bytes: Option<usize>,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut cur = start_sect;
    let total_file_sectors = data.len() / sector_size;
    let max_sectors = fat.len().min(total_file_sectors).max(1);
    let mut visited = std::collections::HashSet::new();
    let mut steps = 0;
    while cur < DIFSECT && steps < max_sectors && visited.insert(cur) {
        let Some(slice) = read_sector(data, sector_size, cur) else {
            break;
        };
        out.extend_from_slice(slice);
        if let Some(limit) = max_bytes {
            if out.len() >= limit {
                out.truncate(limit);
                return out;
            }
        }
        cur = fat.get(cur as usize).copied().unwrap_or(ENDOFCHAIN);
        steps += 1;
    }
    if let Some(limit) = max_bytes {
        if out.len() > limit {
            out.truncate(limit);
        }
    }
    out
}

fn read_mini_chain(
    mini_stream: &[u8],
    mini_sector_size: usize,
    minifat: &[u32],
    target: &DirItem,
) -> Option<Vec<u8>> {
    let bounded_size = target.size.min(mini_stream.len());
    let mut out = Vec::with_capacity(bounded_size);
    let mut cur = target.start_sect;
    let total_mini_sectors = mini_stream.len() / mini_sector_size + 1;
    let max_steps = minifat.len().min(total_mini_sectors).max(1);
    let mut visited = std::collections::HashSet::new();
    let mut steps = 0;
    while cur < DIFSECT && steps < max_steps && out.len() < bounded_size && visited.insert(cur) {
        let off = (cur as usize).checked_mul(mini_sector_size)?;
        let end = (off + mini_sector_size).min(mini_stream.len());
        if off >= mini_stream.len() {
            break;
        }
        out.extend_from_slice(&mini_stream[off..end]);
        cur = minifat.get(cur as usize).copied().unwrap_or(ENDOFCHAIN);
        steps += 1;
    }
    if out.len() > bounded_size {
        out.truncate(bounded_size);
    }
    Some(out)
}

fn parse_directory_entries(dir_bytes: &[u8]) -> Vec<DirItem> {
    let mut entries = Vec::new();
    for entry in dir_bytes.as_chunks::<CFB_DIRECTORY_ENTRY_SIZE>().0 {
        let name_len = get_u16(entry, 64).unwrap_or(0) as usize;
        let entry_type = entry[66];
        if entry_type == 0 || !(2..=64).contains(&name_len) {
            continue;
        }
        let u16_count = (name_len / 2).saturating_sub(1);
        let mut utf16 = Vec::with_capacity(u16_count);
        for i in 0..u16_count {
            if let Some(ch) = get_u16(entry, i * 2) {
                if ch == 0 {
                    break;
                }
                utf16.push(ch);
            }
        }
        entries.push(DirItem {
            name: String::from_utf16_lossy(&utf16),
            entry_type,
            start_sect: get_u32(entry, 116).unwrap_or(ENDOFCHAIN),
            size: get_u32(entry, 120).unwrap_or(0) as usize,
        });
    }
    entries
}

fn collect_fat_sector_ids(data: &[u8], header: &CfbHeader) -> Option<Vec<u32>> {
    let mut fat_sects = Vec::new();
    for i in 0..CFB_HEADER_DIFAT_ENTRIES {
        let sect = get_u32(data, 76 + i * 4)?;
        if sect < DIFSECT {
            fat_sects.push(sect);
        }
    }

    let mut difat_sect = header.first_difat_sect;
    let max_difat = header
        .num_difat_sects
        .min(data.len() / header.sector_size + 1);
    let mut visited_difat = std::collections::HashSet::new();
    for _ in 0..max_difat {
        if difat_sect >= DIFSECT || !visited_difat.insert(difat_sect) {
            break;
        }
        let Some(sector) = read_sector(data, header.sector_size, difat_sect) else {
            break;
        };
        let entries = header.sector_size / 4;
        for i in 0..entries.saturating_sub(1) {
            if let Some(sect) = get_u32(sector, i * 4) {
                if sect < DIFSECT {
                    fat_sects.push(sect);
                }
            }
        }
        difat_sect = get_u32(sector, (entries - 1) * 4).unwrap_or(ENDOFCHAIN);
    }
    Some(fat_sects)
}

fn collect_fat(data: &[u8], sector_size: usize, fat_sects: &[u32]) -> Vec<u32> {
    let mut fat = Vec::with_capacity(fat_sects.len() * (sector_size / 4));
    for &sect in fat_sects {
        if let Some(sector) = read_sector(data, sector_size, sect) {
            for entry in sector.as_chunks::<4>().0 {
                fat.push(u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]));
            }
        }
    }
    fat
}

struct CfbReader<'a> {
    data: &'a [u8],
    header: CfbHeader,
    fat: Vec<u32>,
}

impl<'a> CfbReader<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        let header = CfbHeader::parse(data)?;
        let fat_sects = collect_fat_sector_ids(data, &header)?;
        let fat = collect_fat(data, header.sector_size, &fat_sects);
        Some(Self { data, header, fat })
    }

    fn directory_entries(&self) -> Vec<DirItem> {
        let bytes = read_chain(
            self.data,
            self.header.sector_size,
            &self.fat,
            self.header.first_dir_sect,
            None,
        );
        parse_directory_entries(&bytes)
    }

    fn read_regular_stream(&self, stream: &DirItem) -> Vec<u8> {
        read_chain(
            self.data,
            self.header.sector_size,
            &self.fat,
            stream.start_sect,
            Some(stream.size),
        )
    }

    fn mini_fat(&self) -> Vec<u32> {
        if self.header.first_minifat_sect >= DIFSECT || self.header.num_minifat_sects == 0 {
            return Vec::new();
        }
        let bytes = read_chain(
            self.data,
            self.header.sector_size,
            &self.fat,
            self.header.first_minifat_sect,
            Some(self.header.num_minifat_sects * self.header.sector_size),
        );
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|entry| u32::from_le_bytes(*entry))
            .collect()
    }

    fn read_mini_stream(&self, root: &DirItem, stream: &DirItem) -> Option<Vec<u8>> {
        let mini_stream = self.read_regular_stream(root);
        read_mini_chain(
            &mini_stream,
            self.header.mini_sector_size,
            &self.mini_fat(),
            stream,
        )
    }

    fn read_stream(&self, target_names: &[&str]) -> Option<Vec<u8>> {
        let entries = self.directory_entries();
        let root = entries.iter().find(|entry| entry.entry_type == 5)?;
        let target = target_names.iter().find_map(|&candidate| {
            entries
                .iter()
                .find(|entry| entry.entry_type == 2 && entry.name.eq_ignore_ascii_case(candidate))
        })?;

        if target.size == 0 {
            return Some(Vec::new());
        }
        if target.size >= self.header.mini_cutoff {
            return Some(self.read_regular_stream(target));
        }
        self.read_mini_stream(root, target)
    }
}

/// Extract a named stream from an OLE2 Compound File Binary (`data`).
pub fn read_ole_stream(data: &[u8], target_names: &[&str]) -> Option<Vec<u8>> {
    CfbReader::parse(data)?.read_stream(target_names)
}

/// Strip the 28-byte `EQNOLEFILEHDR` prefix from a raw `Equation Native` stream
/// and return the MTEF payload.
pub fn strip_eqn_ole_header(raw: &[u8]) -> Option<&[u8]> {
    if raw.len() < 28 {
        return None;
    }
    let cb_hdr = get_u16(raw, 0)?;
    let cb_object = get_u32(raw, 8)? as usize;
    if cb_hdr == 28 {
        let mtef = &raw[28..];
        if cb_object > 0 && cb_object <= mtef.len() {
            Some(&mtef[..cb_object])
        } else {
            Some(mtef)
        }
    } else if matches!(raw.first(), Some(3 | 5)) {
        Some(raw)
    } else {
        None
    }
}

/// Extract the MTEF byte stream from a MathType OLE compound file.
pub fn extract_mtef_from_ole(ole_data: &[u8]) -> Option<Vec<u8>> {
    let raw = read_ole_stream(ole_data, MTEF_STREAM_NAMES)?;
    strip_eqn_ole_header(&raw).map(|s| s.to_vec())
}

/// Older EqnTide previews sometimes stored bare v5 records whose first tag
/// (3=template or 5=matrix) was mistaken for a stream version. Only repair when
/// wrapping those exact records reproduces the paired OLE's complete MTEF.
pub(crate) fn repair_legacy_metafile_mtef(mtef: Vec<u8>, ole: Option<&[u8]>) -> Vec<u8> {
    if let Some(reference) = ole.and_then(extract_mtef_from_ole) {
        let complete = crate::mtef::build_mtef(&mtef);
        if complete == reference {
            return complete;
        }
    }
    mtef
}

#[cfg(test)]
mod legacy_preview_tests {
    use super::*;

    #[test]
    fn real_upgrade_samples_recover_the_original_equation() {
        for (ole, emf) in [
            (
                &include_bytes!("../tests/fixtures/legacy_preview/angle.ole")[..],
                &include_bytes!("../tests/fixtures/legacy_preview/angle.emf")[..],
            ),
            (
                &include_bytes!("../tests/fixtures/legacy_preview/fraction.ole")[..],
                &include_bytes!("../tests/fixtures/legacy_preview/fraction.emf")[..],
            ),
            (
                &include_bytes!("../tests/fixtures/legacy_preview/system.ole")[..],
                &include_bytes!("../tests/fixtures/legacy_preview/system.emf")[..],
            ),
        ] {
            let original = extract_mtef_from_ole(ole).unwrap();
            let broken = extract_mtef_from_emf(emf).unwrap();
            assert_ne!(broken, original);
            let repaired = repair_legacy_metafile_mtef(broken.clone(), Some(ole));
            assert_eq!(repaired, original);
            assert!(crate::mtef::decode::mtef_to_mathml(&repaired).is_some());
            assert_eq!(repair_legacy_metafile_mtef(broken.clone(), None), broken);
        }
    }

    #[test]
    fn crafted_cfb_headers_and_cycles_fail_cleanly_without_oom_or_hang() {
        // 1. Oversized mini_cutoff + huge directory stream size must fail cleanly.
        let mut bad_cutoff = vec![0u8; 1024];
        bad_cutoff[0..8].copy_from_slice(&MAGIC);
        put_u16(&mut bad_cutoff, 30, 9);
        put_u16(&mut bad_cutoff, 32, 6);
        put_u32(&mut bad_cutoff, 56, u32::MAX);
        assert!(read_ole_stream(&bad_cutoff, MTEF_STREAM_NAMES).is_none());

        // 2. Huge num_difat_sects with self-referencing DIFAT sector 0 must not loop.
        let mut cyclic_difat = vec![0u8; 1024];
        cyclic_difat[0..8].copy_from_slice(&MAGIC);
        put_u16(&mut cyclic_difat, 30, 9);
        put_u16(&mut cyclic_difat, 32, 6);
        put_u32(&mut cyclic_difat, 56, 4096);
        put_u32(&mut cyclic_difat, 68, 0);
        put_u32(&mut cyclic_difat, 72, u32::MAX);
        for i in 0..CFB_HEADER_DIFAT_ENTRIES {
            put_u32(&mut cyclic_difat, 76 + i * 4, FREESECT);
        }
        assert!(read_ole_stream(&cyclic_difat, MTEF_STREAM_NAMES).is_none());

        // 3. Valid make_ole -> extract_mtef_from_ole and MathML decode round-trip stay intact.
        let mml = crate::MathNode::parse("<math><mi>x</mi></math>").unwrap();
        let eq_bytes = crate::mtef::encode::mathml_to_mtef_bytes(&mml).unwrap();
        let native = crate::mtef::make_equation_native(&eq_bytes);
        let ole = make_ole(&native).unwrap();
        let extracted = extract_mtef_from_ole(&ole).unwrap();
        assert_eq!(extracted, native[28..]);
        assert!(crate::mtef::decode::ole_to_mathml(&ole).is_some());

        // 4. Oversized DirItem.size on an otherwise valid OLE is clamped/rejected cleanly.
        let mut oversized_dir = ole.clone();
        let mut patched = false;
        for chunk in oversized_dir.as_chunks_mut::<128>().0 {
            if chunk.starts_with(b"E\0q\0u\0a\0t\0i\0o\0n\0 \0N\0a\0t\0i\0v\0e\0") {
                put_u32(chunk, 120, u32::MAX);
                patched = true;
            }
        }
        assert!(patched);
        let _ = read_ole_stream(&oversized_dir, MTEF_STREAM_NAMES);
    }
}

const WMF_PLACEABLE: [u8; 4] = [0xD7, 0xCD, 0xC6, 0x9A];
pub const WMF_SIGNATURE: &[u8] = b"Design Science, Inc.\x00";

/// Extract the MTEF byte stream embedded inside a MathType EMF picture
/// (`EMR_COMMENT` with `"AppsMFCC"` signature), if any.
pub fn extract_mtef_from_emf(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 88 {
        return None;
    }
    // Verify EMF signature at offset 40 (dSignature == 0x464D_4520 " EMF") and record type 1 (EMR_HEADER)
    if get_u32(data, 0)? != 1 || get_u32(data, 40)? != 0x464D_4520 {
        return None;
    }

    let mut pos = 0;
    let mut total: Option<usize> = None;
    let mut payload = Vec::new();

    while pos + 8 <= data.len() {
        let kind = get_u32(data, pos)?;
        let size = get_u32(data, pos + 4)? as usize;
        if size < 8 || !size.is_multiple_of(4) {
            break;
        }
        let next_pos = pos.checked_add(size)?;
        if next_pos > data.len() {
            break;
        }

        if kind == 0x0000_0046 && size >= 12 {
            // EMR_COMMENT record:
            // offset + 8: cbData (u32)
            // offset + 12: CommentData (cbData bytes)
            let cb_data = get_u32(data, pos + 8)? as usize;
            let data_end = (pos + 12 + cb_data).min(next_pos);
            if pos + 12 <= data_end {
                let comment_data = &data[pos + 12..data_end];
                if let Some(app_off) = comment_data.windows(8).position(|w| w == b"AppsMFCC") {
                    let chunk = &comment_data[app_off..];
                    if chunk.len() >= 18 {
                        let size_total = get_u32(chunk, 10)? as usize;
                        let size_chunk = get_u32(chunk, 14)? as usize;
                        let mut body = &chunk[18..];
                        if body.starts_with(WMF_SIGNATURE) {
                            body = &body[WMF_SIGNATURE.len()..];
                        }
                        if total.is_none() {
                            total = Some(size_total);
                        }
                        let take = if size_chunk > 0 && size_chunk <= body.len() {
                            &body[..size_chunk]
                        } else {
                            body
                        };
                        payload.extend_from_slice(take);
                    }
                }
            }
        } else if kind == 14 {
            // EMR_EOF record
            break;
        }

        pos = next_pos;
    }

    if payload.is_empty() {
        return None;
    }
    if let Some(t) = total {
        if t > 0 && t < payload.len() {
            payload.truncate(t);
        }
    }
    if matches!(payload.first(), Some(3 | 5)) {
        Some(payload)
    } else {
        None
    }
}

/// Extract the MTEF byte stream embedded inside a MathType WMF picture
/// (`META_ESCAPE` / `MFCOMMENT` with `"AppsMFCC"` signature), if any.
pub fn extract_mtef_from_wmf(data: &[u8]) -> Option<Vec<u8>> {
    let off = if data.starts_with(&WMF_PLACEABLE) {
        22
    } else {
        0
    };
    let mut pos = off + 18; // skip standard METAHEADER
    let mut total: Option<usize> = None;
    let mut payload = Vec::new();

    while pos + 6 <= data.len() {
        let size = get_u32(data, pos)? as usize;
        let func = get_u16(data, pos + 4)?;
        if size < 3 {
            break;
        }
        if func == 0x0626 && pos + 10 <= data.len() {
            // META_ESCAPE
            let esc = get_u16(data, pos + 6)?;
            let count = get_u16(data, pos + 8)? as usize;
            let chunk_end = (pos + 10 + count).min(data.len());
            let chunk = &data[pos + 10..chunk_end];
            if esc == 15 && chunk.len() >= 18 && &chunk[..8] == b"AppsMFCC" {
                let size_total = get_u32(chunk, 10)? as usize;
                let size_chunk = get_u32(chunk, 14)? as usize;
                let mut body = &chunk[18..];
                if body.starts_with(WMF_SIGNATURE) {
                    body = &body[WMF_SIGNATURE.len()..];
                }
                if total.is_none() {
                    total = Some(size_total);
                }
                let take = if size_chunk > 0 && size_chunk <= body.len() {
                    &body[..size_chunk]
                } else {
                    body
                };
                payload.extend_from_slice(take);
            }
        }
        pos = pos.checked_add(size.checked_mul(2)?)?;
    }

    if payload.is_empty() {
        return None;
    }
    if let Some(t) = total {
        if t > 0 && t < payload.len() {
            payload.truncate(t);
        }
    }
    if matches!(payload.first(), Some(3 | 5)) {
        Some(payload)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mtef_from_emf_roundtrip() {
        let node = crate::mathml::MathNode::with_children(
            "math",
            vec![crate::mathml::MathNode::leaf("mi", "x")],
        );
        let preview = crate::render::emf::render_emf(&node, 12.0).unwrap();
        let extracted = extract_mtef_from_emf(&preview.emf).expect("MTEF extracted from EMF");
        assert_eq!(extracted[0], 5, "MTEF version should be 5");
        let mathml = crate::mtef::decode::mtef_to_mathml(&extracted).expect("decodes to mathml");
        assert_eq!(mathml.children[0].children[0].text, "x");
    }

    #[test]
    fn test_make_and_extract_ole_roundtrip_small_and_large() {
        // Small stream (< 4096 bytes -> MiniStream)
        let eq_small = vec![
            0x05, 0x00, 0x00, 0x05, 0x00, b'D', b'S', b'M', b'T', b'5', 0x00, 0x00, 0x0A, 0x01,
            0x00, 0x00,
        ];
        let native_small = crate::mtef::make_equation_native(&[0x02, 0x00, 0x83, b'x', 0x00]);
        let ole_small = make_ole(&native_small).unwrap();
        let extracted_small = extract_mtef_from_ole(&ole_small).unwrap();
        assert_eq!(extracted_small[0], 5);
        assert!(extracted_small.len() > 12);
        let _ = eq_small;

        // Large stream (>= 4096 bytes -> regular FAT sectors)
        let large_payload = vec![0x02; 5000];
        let native_large = crate::mtef::make_equation_native(&large_payload);
        let ole_large = make_ole(&native_large).unwrap();
        let extracted_large = extract_mtef_from_ole(&ole_large).unwrap();
        assert_eq!(extracted_large.len(), native_large.len() - 28);
    }

    #[test]
    fn reads_equation_native_at_the_mini_stream_boundary() {
        for payload_len in [MINI_CUTOFF - 29, MINI_CUTOFF - 28] {
            let payload = vec![0xA5; payload_len];
            let native = crate::mtef::make_equation_native(&payload);
            let ole = make_ole(&native).unwrap();

            assert_eq!(read_ole_stream(&ole, MTEF_STREAM_NAMES), Some(native));
            assert_eq!(
                extract_mtef_from_ole(&ole),
                Some(crate::mtef::build_mtef(&payload))
            );
        }
    }

    #[test]
    fn reads_streams_across_fat_and_mini_fat_chains() {
        let mini_payload = vec![0x11; MINI_SECTOR * 3 + 7];
        let large_payload = vec![0x22; SECTOR * 3 + 19];
        for payload in [&mini_payload, &large_payload] {
            let native = crate::mtef::make_equation_native(payload);
            let ole = make_ole(&native).unwrap();

            assert_eq!(read_ole_stream(&ole, MTEF_STREAM_NAMES), Some(native));
            assert_eq!(
                extract_mtef_from_ole(&ole),
                Some(crate::mtef::build_mtef(payload))
            );
        }
    }

    #[test]
    fn rejects_malformed_compound_files_without_panicking() {
        assert_eq!(read_ole_stream(&[], MTEF_STREAM_NAMES), None);
        assert_eq!(read_ole_stream(&MAGIC, MTEF_STREAM_NAMES), None);

        let native = crate::mtef::make_equation_native(&[5, 0, 0]);
        let mut ole = make_ole(&native).unwrap();
        ole[0] = 0;
        assert_eq!(read_ole_stream(&ole, MTEF_STREAM_NAMES), None);

        let truncated = &make_ole(&native).unwrap()[..SECTOR];
        assert_eq!(read_ole_stream(truncated, MTEF_STREAM_NAMES), None);
    }
}
