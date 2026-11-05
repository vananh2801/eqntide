//! Tiny PNG writer for black-on-transparent equation pictures (4-bit indexed).
//!
//! The previews are black glyphs on a transparent background, so only an opacity
//! level per pixel is stored.

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut h = crc32fast::Hasher::new();
    h.update(kind);
    h.update(data);
    out.extend_from_slice(&h.finalize().to_be_bytes());
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (a as i32, b as i32, c as i32);
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Apply PNG filter `t` (1 byte per pixel unit) to `row` given the previous row.
fn filter_row(t: u8, row: &[u8], prev: &[u8]) -> Vec<u8> {
    (0..row.len())
        .map(|i| {
            let a = if i > 0 { row[i - 1] } else { 0 };
            let b = prev[i];
            let c = if i > 0 { prev[i - 1] } else { 0 };
            let x = row[i];
            match t {
                1 => x.wrapping_sub(a),
                2 => x.wrapping_sub(b),
                3 => x.wrapping_sub(((a as u16 + b as u16) / 2) as u8),
                4 => x.wrapping_sub(paeth(a, b, c)),
                _ => x,
            }
        })
        .collect()
}

/// Number of opacity levels stored (8-bit indexed PNG).
pub const LEVELS: usize = 256;

/// Encode `alpha` (row-major, `width × height`) as an 8-bit black, transparent-background PNG.
pub fn encode_alpha(width: u32, height: u32, alpha: &[u8], pixels_per_metre: u32) -> Vec<u8> {
    assert_eq!(alpha.len(), (width * height) as usize);
    let w = width as usize;
    let h = height as usize;

    let mut packed: Vec<Vec<u8>> = Vec::with_capacity(h);
    for row in alpha.chunks(w) {
        packed.push(row.to_vec());
    }

    // Adaptive filtering: per row, keep the filter with the smallest sum of |value|.
    let zero = vec![0u8; w];
    let mut raw = Vec::with_capacity((w + 1) * h);
    for (y, row) in packed.iter().enumerate() {
        let prev = if y > 0 { &packed[y - 1] } else { &zero };
        let best = (0u8..=4)
            .map(|t| {
                let f = filter_row(t, row, prev);
                let cost: u64 = f.iter().map(|&v| (v as i8).unsigned_abs() as u64).sum();
                (cost, t, f)
            })
            .min_by_key(|(c, _, _)| *c)
            .expect("five candidates");
        raw.push(best.1);
        raw.extend_from_slice(&best.2);
    }

    let mut z = ZlibEncoder::new(Vec::new(), Compression::best());
    z.write_all(&raw).expect("writing to a Vec cannot fail");
    let idat = z.finish().expect("writing to a Vec cannot fail");

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());

    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);

    chunk(&mut out, b"PLTE", &[0u8; LEVELS * 3]); // every entry black
    let trns: Vec<u8> = (0..LEVELS).map(|i| (i * 255 / (LEVELS - 1)) as u8).collect();
    chunk(&mut out, b"tRNS", &trns);

    let mut phys = Vec::with_capacity(9);
    phys.extend_from_slice(&pixels_per_metre.to_be_bytes());
    phys.extend_from_slice(&pixels_per_metre.to_be_bytes());
    phys.push(1); // unit: metre
    chunk(&mut out, b"pHYs", &phys);

    chunk(&mut out, b"IDAT", &idat);
    chunk(&mut out, b"IEND", &[]);
    out
}
