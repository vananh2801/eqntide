//! Native palette backed by the shared conversion catalog.
#[allow(unused_imports)]
pub use eqntide::catalog::{
    attaches_to_previous, matrix, preview_template, template, GROUPS, PALETTE,
};
use eqntide::MathNode;

pub const RECENT_GROUP: &str = "Recent";
pub const DEFAULT_GROUP: &str = "Fractions";

pub type PaletteItem = (&'static str, &'static str, &'static str);

#[derive(Clone, Copy, Debug)]
pub struct UiGroup {
    pub id: &'static str,
    pub label: &'static str,
    pub icon_source: &'static str,
    pub catalog_groups: &'static [&'static str],
}

pub const UI_GROUPS: &[UiGroup] = &[
    UiGroup {
        id: RECENT_GROUP,
        label: "Recent",
        icon_source: "★",
        catalog_groups: &[],
    },
    UiGroup {
        id: "Fractions",
        label: "Fractions & Radicals",
        icon_source: r"\frac{#?}{#?}",
        catalog_groups: &["Fractions"],
    },
    UiGroup {
        id: "Scripts",
        label: "Scripts",
        icon_source: "x^{#?}",
        catalog_groups: &["Scripts"],
    },
    UiGroup {
        id: "Fences",
        label: "Fences",
        icon_source: r"\left( #? \right)",
        catalog_groups: &["Fences"],
    },
    UiGroup {
        id: "Integrals",
        label: "Integrals",
        icon_source: r"\int #?",
        catalog_groups: &["Integrals"],
    },
    UiGroup {
        id: "SumsAndBigOps",
        label: "Sums & Big Operators",
        icon_source: r"\sum_{#?}^{#?}",
        catalog_groups: &["Sums", "BigOperators"],
    },
    UiGroup {
        id: "MatricesAndBoxes",
        label: "Matrices & Boxes",
        icon_source: r"\begin{pmatrix} #? & #? \\ #? & #? \end{pmatrix}",
        catalog_groups: &["Matrices", "Boxes"],
    },
    UiGroup {
        id: "AccentsAndBars",
        label: "Accents & Bars",
        icon_source: r"\bar{#?}",
        catalog_groups: &["Embellishments", "Bars"],
    },
    UiGroup {
        id: "OperatorsAndRelations",
        label: "Operators & Relations",
        icon_source: r"\pm\leq",
        catalog_groups: &["Operators", "Relations"],
    },
    UiGroup {
        id: "Arrows",
        label: "Arrows",
        icon_source: r"\rightarrow\Rightarrow",
        catalog_groups: &["Arrows", "LabeledArrows"],
    },
    UiGroup {
        id: "LogicSetsSymbols",
        label: "Logic, Sets & Symbols",
        icon_source: r"\forall\in",
        catalog_groups: &["Logic", "Sets", "Symbols", "Spaces"],
    },
    UiGroup {
        id: "Greek",
        label: "Greek Letters",
        icon_source: r"\alpha\Omega",
        catalog_groups: &["GreekLower", "GreekUpper"],
    },
];

pub fn ui_group(id: &str) -> Option<&'static UiGroup> {
    UI_GROUPS.iter().find(|group| group.id == id)
}

pub fn canonical_source(source: &str) -> Option<&'static str> {
    PALETTE
        .iter()
        .find(|(_, _, item_source)| *item_source == source)
        .map(|(_, _, item_source)| *item_source)
}

pub fn group_contains(ui_group_id: &str, catalog_group: &str) -> bool {
    ui_group(ui_group_id).is_some_and(|group| group.catalog_groups.contains(&catalog_group))
        || ui_group_id == catalog_group
}

pub fn items_for_ui_group(ui_group_id: &str, recent: &[&'static str]) -> Vec<&'static PaletteItem> {
    if ui_group_id == RECENT_GROUP {
        return recent
            .iter()
            .filter_map(|&source| {
                PALETTE
                    .iter()
                    .find(|(_, _, item_source)| *item_source == source)
            })
            .collect();
    }
    if let Some(group) = ui_group(ui_group_id) {
        let mut items = Vec::new();
        for &catalog_group in group.catalog_groups {
            items.extend(
                PALETTE
                    .iter()
                    .filter(|(item_group, _, _)| *item_group == catalog_group),
            );
        }
        return items;
    }
    PALETTE
        .iter()
        .filter(|(item_group, _, _)| *item_group == ui_group_id)
        .collect()
}

pub fn icon_math(group: &UiGroup) -> Result<MathNode, String> {
    if group.id == RECENT_GROUP {
        return Ok(MathNode::with_children(
            "math",
            vec![MathNode::leaf("mo", group.icon_source)],
        ));
    }
    preview_template(group.icon_source)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_existing_palette_button_has_a_native_template() {
        let errors: Vec<_> = PALETTE
            .iter()
            .filter_map(|(_, label, source)| {
                template(source).err().map(|e| format!("{label}: {e}"))
            })
            .collect();
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }
    #[test]
    fn every_thumbnail_contains_visible_math_inside_a_math_root() {
        for (_, label, source) in PALETTE {
            let math = preview_template(source).unwrap();
            assert_eq!(math.tag, "math", "{label}");
            let preview = eqntide::render::render_preview(&math, 12.0).unwrap();
            let pixels = image::load_from_memory(&preview.png).unwrap().to_rgba8();
            assert!(
                pixels.pixels().any(|p| p.0[3] > 0),
                "empty thumbnail: {label}"
            );
        }
        let fraction = preview_template("\\frac{#?}{#?}").unwrap();
        assert_eq!(fraction.children[0].tag, "mfrac");
    }

    #[test]
    fn filled_palette_templates_render_and_export_to_office() {
        fn fill(n: &mut MathNode) {
            if matches!(n.tag.as_str(), "mrow" | "mtd")
                && n.children.is_empty()
                && n.text.is_empty()
            {
                n.children.push(MathNode::leaf("mi", "x"));
            }
            for child in &mut n.children {
                fill(child);
            }
        }
        for (_, label, source) in PALETTE {
            let mut node = template(source).unwrap();
            fill(&mut node);
            let math = MathNode::with_children("math", vec![node]);
            let preview = eqntide::render::render_interactive(&math, 12.0)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            assert!(!preview.preview.png.is_empty(), "{label}");
            assert!(
                eqntide::mathml_to_omml_xml(&math)
                    .unwrap()
                    .contains("oMath"),
                "{label}"
            );
            assert!(
                !eqntide::render_emf(&math, 12.0).unwrap().emf.is_empty(),
                "{label}"
            );
        }
    }

    #[test]
    fn ui_groups_cover_every_catalog_group_and_render_visible_icons() {
        assert_eq!(UI_GROUPS.len(), 12);
        assert!(ui_group(DEFAULT_GROUP).is_some());
        assert!(!items_for_ui_group(DEFAULT_GROUP, &[]).is_empty());

        let mut mapped = Vec::new();
        for group in UI_GROUPS {
            let math = icon_math(group).unwrap_or_else(|e| panic!("{}: {e}", group.id));
            assert_eq!(math.tag, "math", "{}", group.id);
            let mask = eqntide::render_alpha_fit(&math, 34.0, 22.0)
                .unwrap_or_else(|e| panic!("{}: {e}", group.id));
            assert!(
                mask.width > 0 && mask.height > 0 && mask.alpha.iter().any(|&a| a > 0),
                "empty sidebar icon for {}",
                group.id
            );
            mapped.extend_from_slice(group.catalog_groups);
        }

        for &(canonical, _) in GROUPS {
            let occurrences = mapped.iter().filter(|&&g| g == canonical).count();
            assert_eq!(
                occurrences, 1,
                "canonical group {canonical} mapped {occurrences} times"
            );
        }
        assert!(group_contains("MatricesAndBoxes", "Matrices"));
        assert!(!group_contains("Fractions", "Matrices"));
        assert!(items_for_ui_group(RECENT_GROUP, &[r"\frac{#?}{#?}", r"\alpha"]).len() == 2);
    }

    #[test]
    fn canonical_source_resolves_valid_unknown_and_duplicate_entries() {
        let valid_owned = String::from(r"\frac{#?}{#?}");
        let resolved = canonical_source(&valid_owned).expect("valid palette source");
        assert_eq!(resolved, r"\frac{#?}{#?}");

        let duplicate_owned = String::from(r"\frac{#?}{#?}");
        let resolved_again = canonical_source(&duplicate_owned).expect("duplicate palette source");
        assert!(std::ptr::eq(resolved, resolved_again));

        assert_eq!(canonical_source(r"\not_a_real_palette_entry"), None);
        assert_eq!(canonical_source(""), None);

        // Every palette entry resolves to itself and repeated lookups return the same canonical slice.
        for (_, _, source) in PALETTE {
            let owned = (*source).to_string();
            let actual = canonical_source(&owned).unwrap();
            let actual_again = canonical_source(&owned.clone()).unwrap();
            assert_eq!(actual, *source);
            assert!(std::ptr::eq(actual, actual_again));
        }
    }
}
