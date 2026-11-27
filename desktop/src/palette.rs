//! Native palette backed by the shared conversion catalog.

use eqntide::catalog::ENTRIES;
#[allow(unused_imports)]
pub use eqntide::catalog::{
    attaches_to_previous, matrix, preview_template, template, GROUPS, PALETTE,
};
use serde::Deserialize;

/// Category identifier for the most-recently-used palette items.
pub const RECENT_GROUP: &str = "Recent";
/// Default sidebar category selected on first launch.
pub const DEFAULT_GROUP: &str = "Fractions";
/// Tuple of `(catalog_group, tooltip_label, latex_template_source)`.
pub type PaletteItem = (&'static str, &'static str, &'static str);

/// Embedded metadata JSON for the pre-rasterized palette sprite atlas.
pub(crate) const ATLAS_JSON: &str =
    include_str!("../../assets/icons/generated/palette-atlas/atlas.json");

/// Metadata describing the packed pre-rasterized palette sprite atlas.
#[derive(Debug, Deserialize)]
pub(crate) struct AtlasMetadata {
    /// Total atlas texture width in physical pixels.
    pub(crate) atlas_width: u32,
    /// Total atlas texture height in physical pixels.
    pub(crate) atlas_height: u32,
    /// Logical icon width in DIP.
    pub(crate) icon_width: f32,
    /// Logical icon height in DIP.
    pub(crate) icon_height: f32,
    /// Per-entry pixel coordinates and catalog source mappings.
    pub(crate) entries: Vec<AtlasEntryMetadata>,
}

/// Pixel bounds and catalog source mapping for one palette icon in [`AtlasMetadata`].
#[derive(Debug, Deserialize)]
pub(crate) struct AtlasEntryMetadata {
    /// Canonical LaTeX template source string.
    pub(crate) source: String,
    /// Left pixel coordinate inside the atlas texture.
    pub(crate) x: u32,
    /// Top pixel coordinate inside the atlas texture.
    pub(crate) y: u32,
    /// Cell width in physical pixels.
    #[serde(rename = "w")]
    pub(crate) width: u32,
    /// Cell height in physical pixels.
    #[serde(rename = "h")]
    pub(crate) height: u32,
}

impl AtlasMetadata {
    /// Parses the embedded [`ATLAS_JSON`] metadata manifest.
    pub(crate) fn load_embedded() -> Self {
        serde_json::from_str(ATLAS_JSON).expect("bundled palette atlas.json is valid")
    }
}

/// Sidebar category descriptor grouping one or more catalog sections.
#[derive(Clone, Copy, Debug)]
pub struct UiGroup {
    pub id: &'static str,
    pub label: &'static str,
    pub icons: &'static [&'static str],
    pub catalog_groups: &'static [&'static str],
}

/// Visual subgroup of palette items partitioned into intra-subgroup column runs.
///
/// Each run starts at row 0 of a fresh column inside the same subgroup and wraps
/// at the ribbon's three-row limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteSubgroup {
    /// Column runs packed left to right within the subgroup.
    pub runs: Vec<Vec<&'static PaletteItem>>,
}

impl PaletteSubgroup {
    fn single(items: Vec<&'static PaletteItem>) -> Self {
        Self { runs: vec![items] }
    }

    /// Returns `true` when every column run in the subgroup is empty.
    pub fn is_empty(&self) -> bool {
        self.runs.iter().all(Vec::is_empty)
    }

    /// Returns the total item count across all column runs in the subgroup.
    pub fn len(&self) -> usize {
        self.runs.iter().map(Vec::len).sum()
    }

    /// Iterates over all items across the subgroup's column runs in display order.
    pub fn iter(&self) -> impl Iterator<Item = &'static PaletteItem> + '_ {
        self.runs.iter().flat_map(|run| run.iter().copied())
    }
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

/// Collects the palette items belonging to `ui_group_id` grouped into visual subgroups.
///
/// `Recent` returns a single continuous subgroup so MRU items pack without column breaks.
pub fn subgroups_for_ui_group(ui_group_id: &str, recent: &[&'static str]) -> Vec<PaletteSubgroup> {
    if ui_group_id == RECENT_GROUP {
        let items: Vec<_> = recent
            .iter()
            .filter_map(|&source| {
                PALETTE
                    .iter()
                    .find(|(_, _, item_source)| *item_source == source)
            })
            .collect();
        return if items.is_empty() {
            Vec::new()
        } else {
            vec![PaletteSubgroup::single(items)]
        };
    }
    let mut subgroups = Vec::new();
    if let Some(group) = ui_group(ui_group_id) {
        for &catalog_group in group.catalog_groups {
            append_ordered_subgroups(&mut subgroups, catalog_group);
        }
    } else {
        append_ordered_subgroups(&mut subgroups, ui_group_id);
    }
    subgroups
}

/// Collects the flattened palette items belonging to `ui_group_id` (or MRU items for `Recent`).
pub fn items_for_ui_group(ui_group_id: &str, recent: &[&'static str]) -> Vec<&'static PaletteItem> {
    subgroups_for_ui_group(ui_group_id, recent)
        .iter()
        .flat_map(PaletteSubgroup::iter)
        .collect()
}

fn append_ordered_subgroups(subgroups: &mut Vec<PaletteSubgroup>, group: &str) {
    // The catalog emits ENTRIES and PALETTE in the same order.
    let entries: Vec<_> = ENTRIES
        .iter()
        .zip(PALETTE)
        .filter(|(entry, _)| entry.group == group)
        .collect();
    let specs = palette_subgroups(group);
    if specs.is_empty() {
        if !entries.is_empty() {
            subgroups.push(PaletteSubgroup::single(
                entries.into_iter().map(|(_, item)| item).collect(),
            ));
        }
        return;
    }

    let mut assigned = vec![false; entries.len()];
    for &spec in specs {
        let mut runs = Vec::new();
        for run_spec in spec.split('|') {
            let keys: Vec<_> = run_spec.split_whitespace().collect();
            let mut run_entries: Vec<(usize, &'static PaletteItem)> = Vec::new();
            for (idx, (entry, item)) in entries.iter().enumerate() {
                if assigned[idx] {
                    continue;
                }
                let key = palette_order_key(group, entry.id);
                if let Some(position) = keys.iter().position(|&ordered| ordered == key) {
                    assigned[idx] = true;
                    run_entries.push((position, *item));
                }
            }
            if !run_entries.is_empty() {
                // Rust sort_by_key is stable, preserving catalog order for shared-key variants.
                run_entries.sort_by_key(|(position, _)| *position);
                runs.push(run_entries.into_iter().map(|(_, item)| item).collect());
            }
        }
        if !runs.is_empty() {
            subgroups.push(PaletteSubgroup { runs });
        }
    }

    let remaining: Vec<_> = entries
        .into_iter()
        .enumerate()
        .filter_map(|(idx, (_, item))| (!assigned[idx]).then_some(item))
        .collect();
    if !remaining.is_empty() {
        subgroups.push(PaletteSubgroup::single(remaining));
    }
}

fn palette_order_key<'a>(group: &str, id: &'a str) -> &'a str {
    let key = id.split_once('-').map_or(id, |(_, key)| key);
    if matches!(group, "Sums" | "BigOperators") {
        return key.split_once('-').map_or(key, |(family, _)| family);
    }
    if group == "Integrals" {
        return key.split_once('-').map_or(key, |(family, _)| family);
    }
    key
}

fn palette_subgroups(group: &str) -> &'static [&'static str] {
    match group {
        "Scripts" => &[
            "sup sub subsup \
             pre-sup-post-none pre-sub-post-none pre-both-post-none",
            "pre-sup-post-sup pre-sub-post-sup pre-both-post-sup \
             pre-sub-post-sub pre-sup-post-sub pre-both-post-sub \
             pre-both-post-both pre-sub-post-both pre-sup-post-both",
            "overset underset overunderset",
        ],
        "Fractions" => &[
            "frac tfrac dfrac linear small-linear sfrac",
            "sqrt root",
            "long-division binom",
        ],
        "Fences" => &[
            "paren-both paren-left paren-right \
             bracket-both bracket-left bracket-right \
             double-bracket-both double-bracket-left double-bracket-right \
             brace-both brace-left brace-right \
             angle-both angle-left angle-right \
             bar-both bar-left bar-right \
             norm-both norm-left norm-right \
             floor-both floor-left floor-right \
             ceiling-both ceiling-left ceiling-right",
            "bracket-paren paren-bracket",
            "bracket-left-left bracket-right-right bracket-open-open",
            "bra ket bra-ket",
            "evaluation",
            "cases-two cases-three",
            "system-brace-two system-brace-three",
            "system-bracket-two system-bracket-three",
        ],
        "Integrals" => &[
            "int",
            "iint",
            "iiint",
            "oint oiint oiiint",
            "varointclockwise ointctrclockwise",
        ],
        "Sums" => &["sum"],
        "BigOperators" => &[
            "prod",
            "coprod",
            "bigcup",
            "bigcap",
            "bigvee",
            "bigwedge",
            "bigoplus",
            "bigotimes",
            "bigodot",
        ],
        "Matrices" => &[
            "plain-1x2 plain-1x3 plain-1x4 \
             plain-2x1 plain-3x1 plain-4x1 \
             plain-2x2 plain-3x3 plain-4x4",
            "pmatrix2 pmatrix3",
            "bmatrix2 bmatrix3",
            "vmatrix2 vmatrix3",
            "Vmatrix2 Vmatrix3",
            "dots-row dots-column dots-matrix",
        ],
        "Boxes" => &[
            "box roundedbox",
            "top-left top-right | bottom-left bottom-right",
            "top-bottom-left top-bottom-right | top-left-right bottom-left-right",
        ],
        "Embellishments" => &[
            "dot ddot underdot underddot dddot ddddot",
            "tilde undertilde hat check breve mathring",
            "overleftharpoon underleftharpoon | overrightharpoon underrightharpoon",
            "prime double-prime triple-prime left-prime back-prime",
            "cancel bcancel xcancel strike-horizontal",
        ],
        "Bars" => &[
            "overline overline-label",
            "underline underline-label",
            "doubleoverline doubleoverline-label",
            "doubleunderline doubleunderline-label",
            "widehat widetilde",
            "wideparen underparen",
            "overleftarrow overrightarrow overleftrightarrow \
             underleftarrow underrightarrow underleftrightarrow",
            "overbrace overbrace-label",
            "underbrace underbrace-label",
            "overbracket overbracket-label",
            "underbracket underbracket-label",
        ],
        "Functions" => &[
            "sin cos tan cot sec csc",
            "arcsin arccos arctan arccot arcsec arccsc",
            "sinh cosh tanh coth sech csch",
            "log ln exp",
            "lim limsup liminf min max sup inf",
            "det dim ker hom deg arg gcd lcm sgn mod Pr",
        ],
        "Operators" => &[
            "times cdot div pm mp ast circ bullet diamond",
            "oplus otimes odot amalg dagger ddagger",
        ],
        "Relations" => &[
            "leq geq neq ll gg equiv nleq ngeq",
            "prec succ preceq succeq triangleleft triangleright",
            "sim approx simeq cong asymp propto",
            "doteq triangleq estimates fallingdotseq risingdotseq parallel perp",
        ],
        "Arrows" => &[
            "rightarrow leftarrow leftrightarrow uparrow downarrow updownarrow",
            "Rightarrow Leftarrow Leftrightarrow Uparrow Downarrow Updownarrow",
            "longrightarrow longleftarrow longleftrightarrow Longrightarrow Longleftarrow Longleftrightarrow",
            "nearrow searrow swarrow nwarrow neswarrow nwsearrow",
            "mapsto hookrightarrow hookleftarrow curvearrowright curvearrowleft",
            "rightleftarrows leftrightarrows",
            "rightleftharpoons leftrightharpoons",
            "rightharpoonup leftharpoonup",
            "rightharpoondown leftharpoondown",
        ],
        "LabeledArrows" => &[
            "rightarrow-top rightarrow-bottom rightarrow-both \
             leftarrow-top leftarrow-bottom leftarrow-both \
             leftrightarrow-top leftrightarrow-bottom leftrightarrow-both",
            "Rightarrow-top Rightarrow-bottom Rightarrow-both \
             Leftarrow-top Leftarrow-bottom Leftarrow-both \
             Leftrightarrow-top Leftrightarrow-bottom Leftrightarrow-both",
            "rightharpoonup-top rightharpoonup-bottom rightharpoonup-both \
             leftharpoonup-top leftharpoonup-bottom leftharpoonup-both \
             rightleftharpoons-top rightleftharpoons-bottom rightleftharpoons-both \
             leftrightharpoons-top leftrightharpoons-bottom leftrightharpoons-both",
            "mapsto-top mapsto-bottom mapsto-both \
             rightleftarrows-top rightleftarrows-bottom rightleftarrows-both",
        ],
        "Logic" => &[
            "forall exists nexists neg land lor top bot",
            "vdash dashv models therefore because backepsilon",
        ],
        "Sets" => &[
            "in notin",
            "ni notni",
            "subset subseteq subsetneq nsubset nsubseteq supset supseteq supsetneq nsupseteq",
            "cup cap setminus emptyset",
        ],
        "Symbols" => &[
            "number-set-N number-set-Z number-set-Q number-set-R number-set-C",
            "partial nabla infty ell wp Re Im aleph hbar lambdabar imath jmath mho",
            "angle measuredangle sphericalangle degree prime dprime trprime backprime",
            "triangle square rectangle parallelogram bigcirc lozenge",
            "script-A script-B script-N script-R script-S",
            "fraktur-A fraktur-B fraktur-N fraktur-R fraktur-S",
        ],
        "Spaces" => &[
            "ldots cdots vdots ddots iddots",
            "space-0.083333 space-0.166667 space-0.222222 space-0.277778 space-1 space-2",
        ],
        "GreekLower" => &[
            "alpha beta gamma delta epsilon varepsilon zeta eta theta vartheta iota \
             kappa varkappa lambda mu nu xi omicron pi varpi rho varrho sigma varsigma \
             tau upsilon phi varphi chi psi omega",
        ],
        "GreekUpper" => &[
            "Alpha Beta Gamma Delta Epsilon Zeta Eta Theta varTheta Iota Kappa Lambda \
             Mu Nu Xi Omicron Pi Rho Sigma Tau Upsilon varUpsilon Phi Chi Psi Omega",
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eqntide::MathNode;

    #[test]
    fn desktop_order_references_valid_unique_catalog_keys() {
        for &(group, _) in GROUPS {
            let specs = palette_subgroups(group);
            assert!(
                !specs.is_empty(),
                "expected explicit subgroup layout for catalog group {group}"
            );
            let mut seen = std::collections::HashSet::new();
            for &spec in specs {
                for run_spec in spec.split('|') {
                    let mut keys = run_spec.split_whitespace().peekable();
                    assert!(keys.peek().is_some(), "empty column run in {group}: {spec}");
                    for key in keys {
                        assert!(
                            seen.insert(key),
                            "duplicate palette order key {group}/{key}"
                        );
                        assert!(
                            ENTRIES.iter().any(|entry| {
                                entry.group == group && palette_order_key(group, entry.id) == key
                            }),
                            "unknown palette order key {group}/{key}"
                        );
                    }
                }
            }
            for entry in ENTRIES.iter().filter(|entry| entry.group == group) {
                let key = palette_order_key(group, entry.id);
                assert!(
                    seen.contains(key),
                    "catalog entry {group}/{} (key {key}) missing from palette_subgroups",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn structured_subgroups_keep_families_and_pairs_intact() {
        let fractions = subgroups_for_ui_group("Fractions", &[]);
        let fraction_sizes: Vec<_> = fractions.iter().map(PaletteSubgroup::len).collect();
        assert_eq!(fraction_sizes, vec![6, 2, 2]);

        let scripts = subgroups_for_ui_group("Scripts", &[]);
        let script_sizes: Vec<_> = scripts.iter().map(PaletteSubgroup::len).collect();
        assert_eq!(script_sizes, vec![6, 9, 3]);

        let sums_and_big_ops = subgroups_for_ui_group("SumsAndBigOps", &[]);
        assert_eq!(sums_and_big_ops.len(), 10);
        assert!(sums_and_big_ops.iter().all(|group| group.len() == 5));

        let integrals = subgroups_for_ui_group("Integrals", &[]);
        let integral_sizes: Vec<_> = integrals.iter().map(PaletteSubgroup::len).collect();
        assert_eq!(integral_sizes, vec![5, 5, 5, 9, 6]);

        let matrices = subgroups_for_ui_group("MatricesAndBoxes", &[]);
        let matrix_sizes: Vec<_> = matrices.iter().map(PaletteSubgroup::len).collect();
        assert_eq!(matrix_sizes, vec![9, 2, 2, 2, 2, 3, 2, 4, 4]);
        let box_runs: Vec<Vec<usize>> = matrices[6..]
            .iter()
            .map(|subgroup| subgroup.runs.iter().map(Vec::len).collect())
            .collect();
        assert_eq!(box_runs, vec![vec![2], vec![2, 2], vec![2, 2]]);

        let bars = subgroups_for_ui_group("AccentsAndBars", &[]);
        let bar_sizes: Vec<_> = bars.iter().map(PaletteSubgroup::len).collect();
        assert_eq!(
            bar_sizes,
            vec![6, 6, 4, 5, 4, 2, 2, 2, 2, 2, 2, 6, 2, 2, 2, 2]
        );
        let harpoon_runs: Vec<usize> = bars[2].runs.iter().map(Vec::len).collect();
        assert_eq!(harpoon_runs, vec![2, 2]);

        let functions = subgroups_for_ui_group("Functions", &[]);
        let function_sizes: Vec<_> = functions.iter().map(PaletteSubgroup::len).collect();
        assert_eq!(function_sizes, vec![6, 6, 6, 3, 7, 11]);
    }

    #[test]
    fn recent_subgroups_preserve_single_continuous_mru_group() {
        assert!(subgroups_for_ui_group(RECENT_GROUP, &[]).is_empty());
        let recent = [r"\frac{#?}{#?}", r"\alpha", r"\sum", r"\prod"];
        let subgroups = subgroups_for_ui_group(RECENT_GROUP, &recent);
        assert_eq!(subgroups.len(), 1);
        assert_eq!(
            subgroups[0].iter().map(|item| item.2).collect::<Vec<_>>(),
            recent
        );
        assert_eq!(items_for_ui_group(RECENT_GROUP, &recent).len(), 4);
    }

    #[test]
    fn scripts_group_aligns_matching_positions_across_columns() {
        let scripts = items_for_ui_group("Scripts", &[]);
        let labels: Vec<_> = scripts.iter().map(|item| item.1).collect();
        assert_eq!(
            labels,
            vec![
                "Superscript",
                "Subscript",
                "Subscript and superscript",
                "Left superscript",
                "Left subscript",
                "Left sub/superscript",
                "Left superscript & right superscript",
                "Left subscript & right superscript",
                "Left sub/superscript & right superscript",
                "Left subscript & right subscript",
                "Left superscript & right subscript",
                "Left sub/superscript & right subscript",
                "Left sub/superscript & right sub/superscript",
                "Left subscript & right sub/superscript",
                "Left superscript & right sub/superscript",
                "Overscript label",
                "Underscript label",
                "Overscript and underscript label",
            ]
        );
    }

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
        let icon_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/icons/generated/desktop");
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

    #[test]
    fn embedded_palette_atlas_covers_every_palette_entry_with_valid_uvs() {
        let meta = AtlasMetadata::load_embedded();
        assert_eq!(meta.icon_width, 40.0);
        assert_eq!(meta.icon_height, 26.0);
        assert!(meta.atlas_width > 0 && meta.atlas_height > 0);
        assert_eq!(meta.entries.len(), PALETTE.len());

        let by_source: std::collections::HashMap<&str, &AtlasEntryMetadata> = meta
            .entries
            .iter()
            .map(|entry| (entry.source.as_str(), entry))
            .collect();
        assert_eq!(by_source.len(), PALETTE.len());

        for (_, label, source) in PALETTE {
            let entry = by_source
                .get(*source)
                .unwrap_or_else(|| panic!("missing atlas entry for {label} ({source})"));
            assert!(entry.width > 0 && entry.height > 0, "zero size for {label}");
            assert!(
                entry.x + entry.width <= meta.atlas_width
                    && entry.y + entry.height <= meta.atlas_height,
                "out-of-bounds rect for {label}"
            );
            let u_min = entry.x as f32 / meta.atlas_width as f32;
            let v_min = entry.y as f32 / meta.atlas_height as f32;
            let u_max = (entry.x + entry.width) as f32 / meta.atlas_width as f32;
            let v_max = (entry.y + entry.height) as f32 / meta.atlas_height as f32;
            assert!(
                u_min >= 0.0 && u_max <= 1.0 && u_max > u_min,
                "invalid U bounds for {label}"
            );
            assert!(
                v_min >= 0.0 && v_max <= 1.0 && v_max > v_min,
                "invalid V bounds for {label}"
            );
        }
    }
}
