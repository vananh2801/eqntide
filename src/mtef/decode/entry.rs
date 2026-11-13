//! Stream-version dispatch and final MathML assembly for the MTEF decoder.

use crate::mathml::MathNode;

use super::{
    apply_mover_to_list, convert_objs, parse_v3_object_list, parse_v5_equation, ByteStream, MtefObj,
};

fn skip_stream_header(stream: &mut ByteStream<'_>, version: u8) {
    let _ = stream.uint8();
    let _ = stream.uint8();
    let _ = stream.uint8();
    let _ = stream.uint8();
    let _ = stream.uint8();
    if version == 5 {
        for _ in 0..64 {
            if stream.uint8().unwrap_or(0) == 0 {
                break;
            }
        }
        let _ = stream.uint8();
    }
}

fn parse_with_version(data: &[u8], version: u8) -> Option<Vec<MtefObj>> {
    let mut stream = ByteStream::new(data);
    skip_stream_header(&mut stream, version);
    let objects = if version == 5 {
        parse_v5_equation(&mut stream)
    } else {
        parse_v3_object_list(&mut stream, 0)
    };
    (!objects.is_empty()).then_some(objects)
}

fn stream_version(data: &[u8]) -> Option<u8> {
    matches!(*data.first()?, 3 | 5).then(|| data[0])
}

fn mathml_from_objects(mut objects: Vec<MtefObj>) -> MathNode {
    apply_mover_to_list(&mut objects);
    let children = convert_objs(&objects);
    let root_children = if objects.len() == 1 && matches!(objects[0], MtefObj::Pile { .. }) {
        children
    } else {
        vec![MathNode::with_children("mrow", children)]
    };
    let mut math = MathNode::with_children("math", root_children);
    math.set_attr("display", "block");
    math
}

pub(super) fn decode_to_mathml(data: &[u8]) -> Option<MathNode> {
    if data.len() < 2 {
        return None;
    }
    let version = stream_version(data)?;
    let objects = parse_with_version(data, version)
        .or_else(|| parse_with_version(data, if version == 5 { 3 } else { 5 }))?;
    Some(mathml_from_objects(objects))
}
