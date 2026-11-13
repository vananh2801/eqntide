//! Recursive v3/v5 record-list traversal for the MTEF decoder.

use super::{ByteStream, MtefObj, REC_END};

const MAX_RECORD_DEPTH: usize = 64;

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
        if let Some(object) = super::parse_v5_record(stream, record_type, false, depth + 1) {
            objects.push(object);
        }
    }
    objects
}

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
        if let Some(object) = super::parse_v3_record(stream, record_type, options, depth + 1) {
            objects.push(object);
        }
    }
    objects
}

pub(super) fn is_math_obj(object: &MtefObj) -> bool {
    matches!(
        object,
        MtefObj::Char { .. } | MtefObj::Tmpl { .. } | MtefObj::Pile { .. } | MtefObj::Matrix { .. }
    )
}
