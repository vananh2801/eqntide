//! Shared binary record field readers and recursive v3/v5 object-list traversal.

use super::consts::REC_END;
use super::stream::ByteStream;
use super::{v3, v5, MtefObj};

const MAX_RECORD_DEPTH: usize = 64;

/// Parse a sequence of MTEF v5 records until `REC_END`, EOF, or [`MAX_RECORD_DEPTH`].
pub(super) fn parse_v5_object_list(stream: &mut ByteStream<'_>, depth: usize) -> Vec<MtefObj> {
    if depth > MAX_RECORD_DEPTH {
        return Vec::new();
    }
    let mut objects = Vec::new();
    while stream.remaining() > 0 {
        let Some(record_type) = stream.uint8() else {
            break;
        };
        if record_type == REC_END {
            break;
        }
        if let Some(object) = v5::parse_v5_record(stream, record_type, false, depth + 1) {
            objects.push(object);
        }
    }
    objects
}

/// Parse a sequence of MTEF v3 records until `REC_END`, EOF, or [`MAX_RECORD_DEPTH`].
pub(super) fn parse_v3_object_list(stream: &mut ByteStream<'_>, depth: usize) -> Vec<MtefObj> {
    if depth > MAX_RECORD_DEPTH {
        return Vec::new();
    }
    let mut objects = Vec::new();
    while stream.remaining() > 0 {
        let Some(tag) = stream.uint8() else {
            break;
        };
        let record_type = tag & 0x0F;
        let options = (tag >> 4) & 0x0F;
        if record_type == REC_END {
            break;
        }
        if let Some(object) = v3::parse_v3_record(stream, record_type, options, depth + 1) {
            objects.push(object);
        }
    }
    objects
}

/// Return whether `object` carries visible math content (excluding metadata and size markers).
pub(super) fn is_math_obj(object: &MtefObj) -> bool {
    matches!(
        object,
        MtefObj::Char { .. } | MtefObj::Tmpl { .. } | MtefObj::Pile { .. } | MtefObj::Matrix { .. }
    )
}

/// Map an MTEF pile/matrix alignment code to a CSS/MathML alignment keyword.
pub(super) fn halign_name(code: i8) -> &'static str {
    match code {
        1 => "left",
        2 => "center",
        3 => "right",
        4 => "al",
        5 => "dec",
        _ => "center",
    }
}

/// Skip optional nudge coordinates (`dx`, `dy`, plus extended 16-bit nudge on `-128`).
pub(super) fn parse_nudge(stream: &mut ByteStream<'_>) {
    let shift_x = stream.int8().unwrap_or(0);
    let shift_y = stream.int8().unwrap_or(0);
    if shift_x == -128 && shift_y == -128 {
        let _ = stream.mtef16();
        let _ = stream.mtef16();
    }
}

/// Skip a ruler record's tab-stop entries.
pub(super) fn parse_ruler(stream: &mut ByteStream<'_>) {
    let stop_count = stream.int8().unwrap_or(0).clamp(0, 20);
    for _ in 0..stop_count {
        let _ = stream.int8();
        let _ = stream.mtef16();
    }
}

/// Skip a `SIZE` record's point-size payload.
pub(super) fn parse_size(stream: &mut ByteStream<'_>) {
    let Some(selector) = stream.int8() else {
        return;
    };
    if selector == 101 {
        let _ = stream.mtef16();
    } else if selector == 100 {
        let _ = stream.uint8();
        let _ = stream.mtef16();
    } else {
        let _ = stream.uint8();
    }
}

/// Skip an `EQN_PREFS` record containing nibble-encoded sizes, spacings, and style fonts.
pub(super) fn parse_eqn_prefs(stream: &mut ByteStream<'_>) {
    let _opts = stream.uint8();
    let sizes_count = stream.uint8().unwrap_or(0).min(30);
    for _ in 0..sizes_count {
        read_nibble_dim(stream);
    }
    stream.align_byte();

    let Some(spaces_count) = stream.uint8() else {
        return;
    };
    for _ in 0..spaces_count.min(50) {
        read_nibble_dim(stream);
    }
    stream.align_byte();

    let Some(styles_count) = stream.uint8() else {
        return;
    };
    for _ in 0..styles_count.min(20) {
        let Some(font_def) = stream.uint8() else {
            break;
        };
        if font_def != 0 {
            let _ = stream.uint8();
        }
    }
}

fn read_nibble_dim(stream: &mut ByteStream<'_>) {
    if stream.nibble().is_none() {
        return;
    }
    for _ in 0..20 {
        let Some(nibble) = stream.nibble() else {
            break;
        };
        if nibble == 0x0F {
            break;
        }
    }
}
