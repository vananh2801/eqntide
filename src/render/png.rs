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

/// Apply PNG filter `t` (1 byte per pixel unit) into `dst` and return its heuristic cost.
/// Stops early once `cost >= cost_limit`.
fn filter_row_into(t: u8, row: &[u8], prev: &[u8], dst: &mut [u8], cost_limit: u64) -> Option<u64> {
    let mut cost: u64 = 0;
    match t {
        0 => {
            for (i, &x) in row.iter().enumerate() {
                dst[i] = x;
                cost += (x as i8).unsigned_abs() as u64;
                if cost >= cost_limit {
                    return None;
                }
            }
        }
        1 => {
            let mut a = 0u8;
            for (i, &x) in row.iter().enumerate() {
                let v = x.wrapping_sub(a);
                a = x;
                dst[i] = v;
                cost += (v as i8).unsigned_abs() as u64;
                if cost >= cost_limit {
                    return None;
                }
            }
        }
        2 => {
            for (i, (&x, &b)) in row.iter().zip(prev.iter()).enumerate() {
                let v = x.wrapping_sub(b);
                dst[i] = v;
                cost += (v as i8).unsigned_abs() as u64;
                if cost >= cost_limit {
                    return None;
                }
            }
        }
        3 => {
            let mut a = 0u8;
            for (i, (&x, &b)) in row.iter().zip(prev.iter()).enumerate() {
                let v = x.wrapping_sub(((a as u16 + b as u16) / 2) as u8);
                a = x;
                dst[i] = v;
                cost += (v as i8).unsigned_abs() as u64;
                if cost >= cost_limit {
                    return None;
                }
            }
        }
        _ => {
            let mut a = 0u8;
            let mut c = 0u8;
            for (i, (&x, &b)) in row.iter().zip(prev.iter()).enumerate() {
                let v = x.wrapping_sub(paeth(a, b, c));
                a = x;
                c = b;
                dst[i] = v;
                cost += (v as i8).unsigned_abs() as u64;
                if cost >= cost_limit {
                    return None;
                }
            }
        }
    }
    Some(cost)
}

/// Number of opacity levels stored (8-bit indexed PNG).
pub const LEVELS: usize = 256;

/// Encode `alpha` (row-major, `width × height`) as an 8-bit black, transparent-background PNG.
pub fn encode_alpha(width: u32, height: u32, alpha: &[u8], pixels_per_metre: u32) -> Vec<u8> {
    assert_eq!(alpha.len(), (width * height) as usize);
    let w = width as usize;
    let h = height as usize;

    // Adaptive filtering: per row, keep the filter with the smallest sum of |value|,
    // reusing two scratch buffers across the whole image.
    let zero = vec![0u8; w];
    let mut best_row = vec![0u8; w];
    let mut cand_row = vec![0u8; w];
    let mut raw = Vec::with_capacity((w + 1) * h);
    let mut prev_row: &[u8] = &zero;
    for row in alpha.chunks_exact(w) {
        let mut best_t = 0u8;
        let mut best_cost = u64::MAX;
        for t in 0u8..=4 {
            if let Some(cost) = filter_row_into(t, row, prev_row, &mut cand_row, best_cost) {
                best_cost = cost;
                best_t = t;
                std::mem::swap(&mut best_row, &mut cand_row);
            }
        }
        raw.push(best_t);
        raw.extend_from_slice(&best_row);
        prev_row = row;
    }

    let mut z = ZlibEncoder::new(Vec::new(), Compression::new(4));
    z.write_all(&raw).expect("writing to a Vec cannot fail");
    let idat = z.finish().expect("writing to a Vec cannot fail");

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());

    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);

    chunk(&mut out, b"PLTE", &[0u8; LEVELS * 3]); // every entry black
    let trns: Vec<u8> = (0..LEVELS)
        .map(|i| (i * 255 / (LEVELS - 1)) as u8)
        .collect();
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
