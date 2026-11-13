//! OpenType MATH and GSUB metadata extraction for the bundled math face.

use std::collections::HashMap;

use super::{RadicalAssembly, RadicalInfo, RadicalPart};

/// Extract radical variants and their vertical assembly recipe from MATH.
pub(super) fn radical_info(math_face: &ttf_parser::Face<'static>) -> RadicalInfo {
    let Some(glyph_id) = math_face.glyph_index('\u{221A}') else {
        return RadicalInfo {
            variants: Vec::new(),
            assembly: None,
        };
    };
    let Some(math_variants) = math_face.tables().math.and_then(|math| math.variants) else {
        return RadicalInfo {
            variants: Vec::new(),
            assembly: None,
        };
    };
    let Some(construction) = math_variants.vertical_constructions.get(glyph_id) else {
        return RadicalInfo {
            variants: Vec::new(),
            assembly: None,
        };
    };

    let variants = construction
        .variants
        .into_iter()
        .map(|variant| (variant.variant_glyph.0, variant.advance_measurement))
        .collect();
    let assembly = construction.assembly.map(|assembly| RadicalAssembly {
        min_connector_overlap: math_variants.min_connector_overlap,
        parts: assembly
            .parts
            .into_iter()
            .map(|part| RadicalPart {
                glyph_id: part.glyph_id.0,
                start_connector: part.start_connector_length,
                end_connector: part.end_connector_length,
                full_advance: part.full_advance,
                is_extender: part.part_flags.extender(),
            })
            .collect(),
    });

    RadicalInfo { variants, assembly }
}

/// Extract optical-size alternate glyph IDs from the GSUB `ssty` feature.
pub(super) fn ssty_map(math_face: &ttf_parser::Face<'static>) -> HashMap<u16, (u16, u16)> {
    let mut result = HashMap::new();
    let Some(gsub) = math_face.tables().gsub else {
        return result;
    };
    let Some(feature) = gsub.features.find(ttf_parser::Tag::from_bytes(b"ssty")) else {
        return result;
    };
    for lookup_index in feature.lookup_indices {
        let Some(lookup) = gsub.lookups.get(lookup_index) else {
            continue;
        };
        for subtable_index in 0..lookup.subtables.len() {
            let Some(ttf_parser::gsub::SubstitutionSubtable::Alternate(alternates)) = lookup
                .subtables
                .get::<ttf_parser::gsub::SubstitutionSubtable>(subtable_index)
            else {
                continue;
            };
            match alternates.coverage {
                ttf_parser::opentype_layout::Coverage::Format1 { glyphs } => {
                    for (coverage_index, glyph) in glyphs.into_iter().enumerate() {
                        insert_alternates(
                            &mut result,
                            glyph.0,
                            alternates.alternate_sets.get(coverage_index as u16),
                        );
                    }
                }
                ttf_parser::opentype_layout::Coverage::Format2 { records } => {
                    for record in records {
                        for offset in 0..=(record.end.0 - record.start.0) {
                            let glyph = record.start.0 + offset;
                            insert_alternates(
                                &mut result,
                                glyph,
                                alternates.alternate_sets.get(record.value + offset),
                            );
                        }
                    }
                }
            }
        }
    }
    result
}

fn insert_alternates(
    target: &mut HashMap<u16, (u16, u16)>,
    glyph: u16,
    alternates: Option<ttf_parser::gsub::AlternateSet<'_>>,
) {
    if let Some(alternates) = alternates {
        let script = alternates
            .alternates
            .get(0)
            .map(|glyph| glyph.0)
            .unwrap_or(glyph);
        let script_script = alternates
            .alternates
            .get(1)
            .map(|glyph| glyph.0)
            .unwrap_or(script);
        target.insert(glyph, (script, script_script));
    }
}
