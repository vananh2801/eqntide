//! Native palette backed by the shared conversion catalog.

#[allow(unused_imports)]
pub use eqntide::catalog::{
    attaches_to_previous, matrix, preview_template, template, GROUPS, PALETTE,
};

/// Category identifier for the most-recently-used palette items.
pub const RECENT_GROUP: &str = "Recent";
/// Default sidebar category selected on first launch.
pub const DEFAULT_GROUP: &str = "Fractions";

/// Tuple of `(catalog_group, tooltip_label, latex_template_source)`.
pub type PaletteItem = (&'static str, &'static str, &'static str);

/// Sidebar category descriptor grouping one or more catalog sections.
#[derive(Clone, Copy, Debug)]
pub struct UiGroup {
    pub id: &'static str,
    pub label: &'static str,
    pub icons: &'static [&'static str],
    pub catalog_groups: &'static [&'static str],
}

/// Ordered sidebar categories displayed in the desktop editor.
pub const UI_GROUPS: &[UiGroup] = &[
    UiGroup {
        id: RECENT_GROUP,
        label: "Recent",
        icons: &["history"],
        catalog_groups: &[],
    },
    UiGroup {
        id: "Fractions",
        label: "Fractions & Radicals",
        icons: &["math-x-divide-y", "square-root-2"],
        catalog_groups: &["Fractions"],
    },
    UiGroup {
        id: "Scripts",
        label: "Scripts",
        icons: &["x-power-y"],
        catalog_groups: &["Scripts"],
    },
    UiGroup {
        id: "Fences",
        label: "Fences",
        icons: &["parentheses"],
        catalog_groups: &["Fences"],
    },
    UiGroup {
        id: "Integrals",
        label: "Integrals",
        icons: &["math-integral"],
        catalog_groups: &["Integrals"],
    },
    UiGroup {
        id: "SumsAndBigOps",
        label: "Sums & Big Operators",
        icons: &["sum"],
        catalog_groups: &["Sums", "BigOperators"],
    },
    UiGroup {
        id: "MatricesAndBoxes",
        label: "Matrices & Boxes",
        icons: &["matrix"],
        catalog_groups: &["Matrices", "Boxes"],
    },
    UiGroup {
        id: "AccentsAndBars",
        label: "Accents & Bars",
        icons: &["overline", "underline"],
        catalog_groups: &["Embellishments", "Bars"],
    },
    UiGroup {
        id: "Functions",
        label: "Functions",
        icons: &["math-sin"],
        catalog_groups: &["Functions"],
    },
    UiGroup {
        id: "OperatorsAndRelations",
        label: "Operators & Relations",
        icons: &["math-equal-lower"],
        catalog_groups: &["Operators", "Relations"],
    },
    UiGroup {
        id: "Arrows",
        label: "Arrows",
        icons: &["arrows-left-right"],
        catalog_groups: &["Arrows"],
    },
    UiGroup {
        id: "LabeledArrows",
        label: "Labeled Arrows",
        icons: &["arrow-autofit-right"],
        catalog_groups: &["LabeledArrows"],
    },
    UiGroup {
        id: "LogicSetsSymbols",
        label: "Logic, Sets & Symbols",
        icons: &["math-avg"],
        catalog_groups: &["Logic", "Sets", "Symbols", "Spaces"],
    },
    UiGroup {
        id: "Greek",
        label: "Greek Letters",
        icons: &["alpha", "omega"],
        catalog_groups: &["GreekLower", "GreekUpper"],
    },
];

/// Looks up a sidebar UI group descriptor by identifier.
pub fn ui_group(id: &str) -> Option<&'static UiGroup> {
    UI_GROUPS.iter().find(|group| group.id == id)
}

/// Resolves an arbitrary template string to its canonical static slice from `PALETTE`.
pub fn canonical_source(source: &str) -> Option<&'static str> {
    PALETTE
        .iter()
        .find(|(_, _, item_source)| *item_source == source)
        .map(|(_, _, item_source)| *item_source)
}

/// Returns `true` when `ui_group_id` owns `catalog_group` or matches it directly.
pub fn group_contains(ui_group_id: &str, catalog_group: &str) -> bool {
    ui_group(ui_group_id).is_some_and(|group| group.catalog_groups.contains(&catalog_group))
        || ui_group_id == catalog_group
}

/// Collects the palette items belonging to `ui_group_id` (or MRU items for `Recent`).
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

#[cfg(test)]
mod tests {
    use super::*;
    use eqntide::MathNode;

    #[test]
    fn every_existing_palette_button_has_a_native_template() {
        let errors: Vec<_> = PALETTE
            .iter()
            .filter_map(|(_, label, source)| {
                template(source).err().map(|err| format!("{label}: {err}"))
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
                pixels.pixels().any(|pixel| pixel.0[3] > 0),
                "empty thumbnail: {label}"
            );
        }
        let fraction = preview_template("\\frac{#?}{#?}").unwrap();
        assert_eq!(fraction.children[0].tag, "mfrac");
    }

    #[test]
    fn filled_palette_templates_render_and_export_to_office() {
        fn fill(node: &mut MathNode) {
            if matches!(node.tag.as_str(), "mrow" | "mtd")
                && node.children.is_empty()
                && node.text.is_empty()
            {
                node.children.push(MathNode::leaf("mi", "x"));
            }
            for child in &mut node.children {
                fill(child);
            }
        }
        for (_, label, source) in PALETTE {
            let mut node = template(source).unwrap();
            fill(&mut node);
            let math = MathNode::with_children("math", vec![node]);
            let preview = eqntide::render::render_interactive(&math, 12.0)
                .unwrap_or_else(|err| panic!("{label}: {err}"));
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
        assert_eq!(UI_GROUPS.len(), 14);
        assert!(ui_group(DEFAULT_GROUP).is_some());
        assert!(!items_for_ui_group(DEFAULT_GROUP, &[]).is_empty());

        let mut mapped = Vec::new();
        let icon_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/icons/rendered");
        for group in UI_GROUPS {
            assert!(
                matches!(group.icons.len(), 1 | 2),
                "{}: expected 1 or 2 sidebar icons, got {}",
                group.id,
                group.icons.len()
            );
            for &icon_name in group.icons {
                let icon_path = icon_dir.join(format!("{icon_name}.png"));
                let pixels = image::open(&icon_path)
                    .unwrap_or_else(|err| {
                        panic!(
                            "{}: failed to open {}: {err}",
                            group.id,
                            icon_path.display()
                        )
                    })
                    .to_rgba8();
                assert!(
                    pixels.width() > 0
                        && pixels.height() > 0
                        && pixels.pixels().any(|pixel| pixel.0[3] > 0),
                    "empty sidebar icon {icon_name} for {}",
                    group.id
                );
            }
            mapped.extend_from_slice(group.catalog_groups);
        }

        for &(canonical, _) in GROUPS {
            let occurrences = mapped.iter().filter(|&&group| group == canonical).count();
            assert_eq!(
                occurrences, 1,
                "canonical group {canonical} mapped {occurrences} times"
            );
        }
        assert!(group_contains("MatricesAndBoxes", "Matrices"));
        assert!(!group_contains("Fractions", "Matrices"));
        let fn_items = items_for_ui_group("Functions", &[]);
        assert!(fn_items.iter().any(|(_, _, src)| *src == r"\sin"));
        assert!(fn_items
            .iter()
            .any(|(_, _, src)| *src == r"\lim_{#? \to #?}"));
        assert!(fn_items.iter().any(|(_, _, src)| *src == r"\min_{#?}"));
        assert!(fn_items.iter().any(|(_, _, src)| *src == r"\max_{#?}"));
        let script_items = items_for_ui_group("Scripts", &[]);
        assert!(!script_items
            .iter()
            .any(|(_, _, src)| matches!(*src, r"\lim_{#? \to #?}" | r"\min_{#?}" | r"\max_{#?}")));
        let arrow_items = items_for_ui_group("Arrows", &[]);
        assert!(!arrow_items.is_empty());
        assert!(arrow_items.iter().all(|(_, _, src)| !src.contains("#?")));
        let labeled_arrow_items = items_for_ui_group("LabeledArrows", &[]);
        assert!(!labeled_arrow_items.is_empty());
        assert!(labeled_arrow_items
            .iter()
            .all(|(_, _, src)| src.contains("#?")));
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
