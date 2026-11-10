//! Builders for the XML that embeds a MathType OLE object in `document.xml`.

use crate::ole::PROG_ID;
use crate::xml::{self, Element, Node};

use super::consts::*;

/// Build an `mc:AlternateContent` element that offers:
///
/// * `mc:Choice` — the MathType OLE object (used when Word can call MathType)
/// * `mc:Fallback` — the original OMML equation (used when MathType is absent;
///   keeps the equation visible in all environments)
///
/// ```text
/// <mc:AlternateContent>
///   <mc:Choice Requires="mtype">
///     <w:r><w:object ...>
///       <v:shape ... o:ole=""/>
///       <o:OLEObject ProgID="Equation.DSMT4" ... r:id="..."/>
///     </w:object></w:r>
///   </mc:Choice>
///   <mc:Fallback><m:oMath> ... original equation ... </m:oMath></mc:Fallback>
/// </mc:AlternateContent>
/// ```
///
/// `Requires="mtype"` references a custom namespace that no standard Word build
/// recognises. Per ECMA-376 MCE rules an unknown `Requires` selects the
/// `Fallback`, so Word always renders the OMML (equations stay visible) while
/// MathType, which scans the raw XML for `Equation.DSMT*` OLE objects, still
/// finds the embedded binary.
pub fn make_ole_xml(
    rel_id: &str,
    shape_id: &str,
    obj_id: &str,
    width_emu: u64,
    height_emu: u64,
    omml_fallback: &Element,
) -> Element {
    // Convert EMU to twips (1 inch = 914400 EMU = 1440 twips).
    let dxa_w = (width_emu * 1440 / 914_400).max(100);
    let dxa_h = (height_emu * 1440 / 914_400).max(100);

    let mut ac = Element::new(Some(NS_MC), Some("mc"), "AlternateContent");

    // ── Primary: MathType OLE ────────────────────────────────────────────────
    let mut choice = Element::new(Some(NS_MC), Some("mc"), "Choice")
        .with_nsdecl(Some("mtype"), MTYPE_NS);
    choice.set_attr(None, None, "Requires", "mtype");

    let mut wr = Element::new(Some(NS_W), Some("w"), "r");
    let mut wobj = Element::new(Some(NS_W), Some("w"), "object");
    wobj.set_attr(Some(NS_W), Some("w"), "dxaOrig", &dxa_w.to_string());
    wobj.set_attr(Some(NS_W), Some("w"), "dyaOrig", &dxa_h.to_string());

    let mut vshape = Element::new(Some(NS_V), Some("v"), "shape");
    vshape.set_attr(None, None, "id", shape_id);
    vshape.set_attr(None, None, "type", "#_x0000_t75");
    vshape.set_attr(
        None,
        None,
        "style",
        &format!(
            "width:{:.1}pt;height:{:.1}pt",
            dxa_w as f64 / 20.0,
            dxa_h as f64 / 20.0
        ),
    );
    vshape.set_attr(Some(NS_O), Some("o"), "ole", "");

    let mut oleobj = Element::new(Some(NS_O), Some("o"), "OLEObject");
    oleobj.set_attr(None, None, "Type", "Embed");
    oleobj.set_attr(None, None, "ProgID", PROG_ID);
    oleobj.set_attr(None, None, "ShapeID", shape_id);
    oleobj.set_attr(None, None, "DrawAspect", "Content");
    oleobj.set_attr(None, None, "ObjectID", obj_id);
    oleobj.set_attr(Some(NS_R), Some("r"), "id", rel_id);

    wobj.push(vshape);
    wobj.push(oleobj);
    wr.push(wobj);
    choice.push(wr);

    // ── Fallback: original OMML (always visible) ─────────────────────────────
    let mut fallback = Element::new(Some(NS_MC), Some("mc"), "Fallback");
    fallback.push(omml_fallback.clone());

    ac.push(choice);
    ac.push(fallback);
    ac
}

pub(super) const SHAPETYPE_XML: &str = r#"<v:shapetype xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" id="_x0000_t75" coordsize="21600,21600" o:spt="75" o:preferrelative="t" path="m@4@5l@4@11@9@11@9@5xe" filled="f" stroked="f"><v:stroke joinstyle="miter"/><v:formulas><v:f eqn="if lineDrawn pixelLineWidth 0"/><v:f eqn="sum @0 1 0"/><v:f eqn="sum 0 0 @1"/><v:f eqn="prod @2 1 2"/><v:f eqn="prod @3 21600 pixelWidth"/><v:f eqn="prod @3 21600 pixelHeight"/><v:f eqn="sum @0 0 1"/><v:f eqn="prod @6 1 2"/><v:f eqn="prod @7 21600 pixelWidth"/><v:f eqn="sum @8 21600 0"/><v:f eqn="prod @7 21600 pixelHeight"/><v:f eqn="sum @10 21600 0"/></v:formulas><v:path o:extrusionok="f" gradientshapeok="t" o:connecttype="rect"/><o:lock v:ext="edit" aspectratio="t"/></v:shapetype>"#;

pub(super) fn shapetype() -> Element {
    let mut e = xml::parse(SHAPETYPE_XML.as_bytes()).expect("static shapetype is well-formed");
    e.nsdecls.clear(); // the serialiser declares v:/o: only where they are missing
    e
}

/// Build the `w:r` Word writes for an inline MathType equation:
///
/// ```text
/// <w:r>
///   <w:rPr><w:position w:val="-8"/></w:rPr>          ← aligns the picture's baseline
///   <w:object w:dxaOrig=".." w:dyaOrig="..">
///     <v:shapetype id="_x0000_t75" …/>
///     <v:shape id=".." type="#_x0000_t75" style="width:..pt;height:..pt" o:ole="">
///       <v:imagedata r:id="rIdImgN" o:title=""/>      ← the preview picture
///     </v:shape>
///     <o:OLEObject Type="Embed" ProgID="Equation.DSMT4" … r:id="rIdOleN"/>
///   </w:object>
/// </w:r>
/// ```
pub fn make_object_run(
    ole_rid: &str,
    img_rid: &str,
    shape_id: &str,
    obj_id: &str,
    width_pt: f32,
    height_pt: f32,
    depth_pt: f32,
) -> Element {
    let dxa_w = ((width_pt * 20.0).round() as i64).max(1);
    let dxa_h = ((height_pt * 20.0).round() as i64).max(1);
    // Half-points; negative lowers the picture so its baseline meets the text's.
    let position = -((depth_pt * 2.0).round() as i64);

    let mut run = Element::new(Some(NS_W), Some("w"), "r");
    if position != 0 {
        let mut rpr = Element::new(Some(NS_W), Some("w"), "rPr");
        let mut pos = Element::new(Some(NS_W), Some("w"), "position");
        pos.set_attr(Some(NS_W), Some("w"), "val", &position.to_string());
        rpr.push(pos);
        run.push(rpr);
    }

    let mut obj = Element::new(Some(NS_W), Some("w"), "object");
    obj.set_attr(Some(NS_W), Some("w"), "dxaOrig", &dxa_w.to_string());
    obj.set_attr(Some(NS_W), Some("w"), "dyaOrig", &dxa_h.to_string());
    obj.push(shapetype());

    let mut shape = Element::new(Some(NS_V), Some("v"), "shape");
    shape.set_attr(None, None, "id", shape_id);
    shape.set_attr(None, None, "type", "#_x0000_t75");
    shape.set_attr(
        None,
        None,
        "style",
        &format!("width:{:.1}pt;height:{:.1}pt", dxa_w as f64 / 20.0, dxa_h as f64 / 20.0),
    );
    shape.set_attr(Some(NS_O), Some("o"), "ole", "");
    let mut img = Element::new(Some(NS_V), Some("v"), "imagedata");
    img.set_attr(Some(NS_R), Some("r"), "id", img_rid);
    img.set_attr(Some(NS_O), Some("o"), "title", "");
    shape.push(img);
    obj.push(shape);

    let mut ole = Element::new(Some(NS_O), Some("o"), "OLEObject");
    ole.set_attr(None, None, "Type", "Embed");
    ole.set_attr(None, None, "ProgID", PROG_ID);
    ole.set_attr(None, None, "ShapeID", shape_id);
    ole.set_attr(None, None, "DrawAspect", "Content");
    ole.set_attr(None, None, "ObjectID", obj_id);
    ole.set_attr(Some(NS_R), Some("r"), "id", ole_rid);
    obj.push(ole);

    run.push(obj);
    run
}

/// Build a zero-width reset run with `w:position w:val="0"` and text U+200C (ZWNJ).
/// When placed immediately after an inline equation, it acts as a baseline anchor so
/// any text subsequently typed by the user does not inherit the negative position (drop)
/// of the preceding equation run.
pub fn make_reset_run() -> Element {
    let mut reset_run = Element::new(Some(NS_W), Some("w"), "r");
    let mut rpr = Element::new(Some(NS_W), Some("w"), "rPr");
    let mut pos = Element::new(Some(NS_W), Some("w"), "position");
    pos.set_attr(Some(NS_W), Some("w"), "val", "0");
    rpr.push(pos);
    reset_run.push(rpr);
    let mut t = Element::new(Some(NS_W), Some("w"), "t");
    t.children.push(Node::Text("\u{200c}".to_string()));
    reset_run.push(t);
    reset_run
}

/// Recursively sanitize corrupted baselines across the document XML:
/// 1. Removes any `<w:position>` from paragraph properties `<w:pPr><w:rPr>` to prevent
///    paragraph-wide baseline leakage to newly typed text or new lines.
/// 2. Removes any negative `<w:position>` from plain text runs (`<w:r>` with `<w:t>` but no equation/drawing/shape)
///    that may have leaked from previously adjacent equations.
pub fn sanitize_document_baselines(el: &mut Element) {
    if el.is(NS_W, "pPr") || el.local == "pPr" {
        for child in &mut el.children {
            if let Node::Element(rpr) = child {
                if rpr.is(NS_W, "rPr") || rpr.local == "rPr" {
                    rpr.children.retain(|n| {
                        !matches!(n, Node::Element(e) if e.is(NS_W, "position") || e.local == "position")
                    });
                }
            }
        }
    }
    if el.is(NS_W, "r") || el.local == "r" {
        let has_text = el.children.iter().any(|n| matches!(n, Node::Element(e) if e.is(NS_W, "t") || e.local == "t"));
        let has_math_or_shape = el.children.iter().any(|n| matches!(n, Node::Element(e) if e.local == "object" || e.local == "drawing" || e.local == "oMath" || e.local == "shape"));
        if has_text && !has_math_or_shape {
            for child in &mut el.children {
                if let Node::Element(rpr) = child {
                    if rpr.is(NS_W, "rPr") || rpr.local == "rPr" {
                        rpr.children.retain(|n| {
                            if let Node::Element(e) = n {
                                if e.is(NS_W, "position") || e.local == "position" {
                                    if let Some(val) = e.attr(Some(NS_W), "val").or_else(|| e.attr(None, "val")) {
                                        if let Ok(num) = val.parse::<i32>() {
                                            if num < 0 {
                                                return false;
                                            }
                                        }
                                    }
                                }
                            }
                            true
                        });
                    }
                }
            }
        }
    }
    for child in &mut el.children {
        if let Node::Element(c) = child {
            sanitize_document_baselines(c);
        }
    }
}
