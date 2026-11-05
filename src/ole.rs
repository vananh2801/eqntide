//! Minimal OLE Compound File Binary (CFB v3, 512-byte sectors) writer.
//!
//! Produces the same container layout MathType itself writes for an equation:
//! four streams (`\x01Ole`, `\x01CompObj`, `\x03ObjInfo`, `Equation Native`),
//! small streams stored in the mini-stream, and the MathType root CLSID
//! `{0002CE03-0000-0000-C000-000000000046}`. The three auxiliary streams are
//! byte-for-byte copies from a genuine MathType 5 file.
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
/// Streams smaller than this live in the mini-stream.
pub const MINI_CUTOFF: usize = 4096;
/// ProgID MathType registers for its equation objects.
pub const PROG_ID: &str = "Equation.DSMT4";

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

fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
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
                mini_fat.push(if i + 1 < n { (first + i + 1) as u32 } else { ENDOFCHAIN });
            }
            mini_stream.extend_from_slice(data);
            mini_stream.resize((first + n) * MINI_SECTOR, 0);
            placed.push(Placed { start: first as u32, size: data.len() });
        } else {
            big_sizes.push(div_ceil(data.len(), SECTOR));
            placed.push(Placed { start: 0, size: data.len() }); // fixed up below
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
    if n_fat > 109 {
        return Err(Error::EquationTooLarge { stream_len: equation_native.len() });
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
            fat[first + i] = if i + 1 < count { (first + i + 1) as u32 } else { ENDOFCHAIN };
        }
    };
    chain(&mut fat, dir_first, n_dir);
    chain(&mut fat, minifat_first, n_minifat);
    chain(&mut fat, ministream_first, n_ministream);
    let mut cursor = big_first;
    let mut big_iter = big_sizes.iter();
    for (i, (_, data)) in streams.iter().enumerate() {
        if data.len() >= MINI_CUTOFF {
            let count = *big_iter.next().expect("one size per big stream");
            chain(&mut fat, cursor, count);
            placed[i].start = cursor as u32;
            cursor += count;
        }
    }

    // ── Header ──────────────────────────────────────────────────────────────
    let mut header = [0u8; SECTOR];
    header[0..8].copy_from_slice(&MAGIC);
    put_u16(&mut header, 24, 0x003E); // minor version
    put_u16(&mut header, 26, 0x0003); // major version (v3)
    put_u16(&mut header, 28, 0xFFFE); // little-endian
    put_u16(&mut header, 30, 9); // sector size 2^9
    put_u16(&mut header, 32, 6); // mini sector size 2^6
    put_u32(&mut header, 44, n_fat as u32); // NumFATSectors
    put_u32(&mut header, 48, dir_first as u32); // FirstDirectorySector
    put_u32(&mut header, 56, MINI_CUTOFF as u32);
    put_u32(
        &mut header,
        60,
        if n_minifat > 0 { minifat_first as u32 } else { ENDOFCHAIN },
    );
    put_u32(&mut header, 64, n_minifat as u32);
    put_u32(&mut header, 68, ENDOFCHAIN); // no DIFAT sectors
    put_u32(&mut header, 72, 0);
    for i in 0..109 {
        put_u32(&mut header, 76 + i * 4, if i < n_fat { i as u32 } else { FREESECT });
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
        if n_ministream > 0 { ministream_first as u32 } else { ENDOFCHAIN },
        mini_stream.len() as u32,
        MATHTYPE_CLSID,
    );
    let entry = |idx: usize, left: u32, right: u32| {
        dir_entry(
            streams[idx].0,
            2,
            1,
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
