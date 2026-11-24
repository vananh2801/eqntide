//! OpenType MATH and GSUB metadata extraction for the bundled math face.

use std::collections::HashMap;

use read_fonts::tables::gsub::{AlternateSubst, ExtensionSubtable, SubstitutionLookup};
use read_fonts::tables::math::{PartFlags, StretchAxis};
use read_fonts::{FontRef, TableProvider};
use skrifa::Tag;

use super::{RadicalAssembly, RadicalInfo, RadicalPart};

/// Extract radical variants and their vertical assembly recipe from MATH.
pub(super) fn radical_info(math_face: &FontRef<'static>) -> RadicalInfo {
    let Some(glyph_id) = skrifa::charmap::Charmap::new(math_face).map('\u{221A}') else {
        return RadicalInfo {
            variants: Vec::new(),
            assembly: None,
        };
    };
    let Some(math_variants) = math_face
        .math()
        .ok()
        .and_then(|math| math.math_variants().ok())
    else {
        return RadicalInfo {
            variants: Vec::new(),
            assembly: None,
        };
    };
    let Some(construction) = math_variants.glyph_construction(glyph_id, StretchAxis::Vertical)
    else {
        return RadicalInfo {
            variants: Vec::new(),
            assembly: None,
        };
    };

    let variants = construction
        .math_glyph_variant_records()
        .iter()
        .map(|variant| {
            (
                variant.variant_glyph().to_u16(),
                variant.advance_measurement().to_u16(),
            )
        })
        .collect();
    let assembly = construction
        .glyph_assembly()
        .and_then(Result::ok)
        .map(|assembly| RadicalAssembly {
            min_connector_overlap: math_variants.min_connector_overlap().to_u16(),
            parts: assembly
                .part_records()
                .iter()
                .map(|part| RadicalPart {
                    glyph_id: part.glyph_id().to_u16(),
                    start_connector: part.start_connector_length().to_u16(),
                    end_connector: part.end_connector_length().to_u16(),
                    full_advance: part.full_advance().to_u16(),
                    is_extender: part.part_flags().contains(PartFlags::EXTENDER_FLAG),
                })
                .collect(),
        });

    RadicalInfo { variants, assembly }
}

/// Extract optical-size alternate glyph IDs from the GSUB `ssty` feature.
pub(super) fn ssty_map(math_face: &FontRef<'static>) -> HashMap<u16, (u16, u16)> {
    let mut result = HashMap::new();
    let Ok(gsub) = math_face.gsub() else {
        return result;
    };
    let Ok(feature_list) = gsub.feature_list() else {
        return result;
    };
    let Ok(lookup_list) = gsub.lookup_list() else {
        return result;
    };

    let ssty_tag = Tag::new(b"ssty");
    let lookups = lookup_list.lookups();

    for (idx, record) in feature_list.feature_records().iter().enumerate() {
        if record.feature_tag() != ssty_tag {
            continue;
        }
        let Ok(feature) = record.feature(feature_list.offset_data()) else {
            continue;
        };
        let _ = idx;
        for lookup_index in feature.lookup_list_indices() {
            let Ok(lookup) = lookups.get(lookup_index.get() as usize) else {
                continue;
            };
            match lookup {
                SubstitutionLookup::Alternate(alt_lookup) => {
                    for subtable in alt_lookup.subtables().iter().flatten() {
                        collect_alternate_subtable(&mut result, subtable);
                    }
                }
                SubstitutionLookup::Extension(ext_lookup) => {
                    for ext in ext_lookup.subtables().iter().flatten() {
                        if let ExtensionSubtable::Alternate(ext_alt) = ext {
                            if let Ok(subtable) = ext_alt.extension() {
                                collect_alternate_subtable(&mut result, subtable);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    result
}

fn collect_alternate_subtable(target: &mut HashMap<u16, (u16, u16)>, subtable: AlternateSubst<'_>) {
    match subtable {
        AlternateSubst::Format1(alt) => {
            let Ok(coverage) = alt.coverage() else {
                return;
            };
            let sets = alt.alternate_sets();
            for (coverage_index, glyph) in coverage.iter().enumerate() {
                let Ok(gid) = u16::try_from(glyph.to_u32()) else {
                    continue;
                };
                if let Ok(set) = sets.get(coverage_index) {
                    insert_alternates_16(target, gid, set.alternate_glyph_ids());
                }
            }
        }
        AlternateSubst::Format2(alt) => {
            let Ok(coverage) = alt.coverage() else {
                return;
            };
            let sets = alt.alternate_sets();
            for (coverage_index, glyph) in coverage.iter().enumerate() {
                let Ok(gid) = u16::try_from(glyph.to_u32()) else {
                    continue;
                };
                if let Ok(set) = sets.get(coverage_index) {
                    let ids = set.alternate_glyph_ids();
                    if let Some(first) = ids.first() {
                        let script = first.get().to_u32() as u16;
                        let script_script = ids
                            .get(1)
                            .map(|g| g.get().to_u32() as u16)
                            .unwrap_or(script);
                        target.insert(gid, (script, script_script));
                    }
                }
            }
        }
    }
}

fn insert_alternates_16(
    target: &mut HashMap<u16, (u16, u16)>,
    glyph: u16,
    alternates: &[read_fonts::types::BigEndian<read_fonts::types::GlyphId16>],
) {
    if let Some(first) = alternates.first() {
        let script = first.get().to_u16();
        let script_script = alternates
            .get(1)
            .map(|g| g.get().to_u16())
            .unwrap_or(script);
        target.insert(glyph, (script, script_script));
    }
}
