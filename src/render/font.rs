//! Embedded fonts and glyph helpers used to draw equation previews.

use std::collections::HashMap;
use std::sync::OnceLock;

use ab_glyph::{Font, FontRef, GlyphId, PxScale};

use super::canvas::Canvas;

static FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Regular.ttf");
static FONT_ITALIC: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Italic.ttf");
static FONT_BOLD: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Bold.ttf");
static FONT_MATH: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoMath-Regular.ttf");

/// OpenType MATH table constants extracted directly from `STIXTwoMath-Regular.ttf`.
///
/// All linear dimensions are stored as fractions of the em square (`value / upem`),
/// so multiplying by `em_px` yields exact pixel coordinates for any point size.
#[derive(Clone, Copy, Debug)]
pub struct MathConstants {
    /// Height of the mathematical axis above the baseline (fraction of em).
    pub axis_height: f32,
    /// Default thickness of fraction rules (fraction of em).
    pub fraction_rule_thickness: f32,
    /// Numerator baseline shift up in display style (fraction of em).
    pub fraction_num_display_style_shift_up: f32,
    /// Numerator baseline shift up in text/script style (fraction of em).
    pub fraction_num_shift_up: f32,
    /// Denominator baseline shift down in display style (fraction of em).
    pub fraction_den_display_style_shift_down: f32,
    /// Denominator baseline shift down in text/script style (fraction of em).
    pub fraction_den_shift_down: f32,
    /// Minimum gap between numerator bottom and fraction rule in display style (fraction of em).
    pub fraction_num_display_style_gap_min: f32,
    /// Minimum gap between numerator bottom and fraction rule in text/script style (fraction of em).
    pub fraction_num_gap_min: f32,
    /// Minimum gap between denominator top and fraction rule in display style (fraction of em).
    pub fraction_den_display_style_gap_min: f32,
    /// Minimum gap between denominator top and fraction rule in text/script style (fraction of em).
    pub fraction_den_gap_min: f32,
    /// Scale factor for script level 1 (e.g. 0.70).
    pub script_percent_scale_down: f32,
    /// Scale factor for script level 2 (e.g. 0.55).
    pub script_script_percent_scale_down: f32,
    /// Superscript shift up in uncramped style (fraction of em).
    pub superscript_shift_up: f32,
    /// Superscript shift up in cramped style (fraction of em).
    pub superscript_shift_up_cramped: f32,
    /// Subscript shift down (fraction of em).
    pub subscript_shift_down: f32,
    /// Minimum clearance between subscript top and superscript bottom (fraction of em).
    pub sub_superscript_gap_min: f32,
    /// Vertical gap between radicand top and radical rule in text style (fraction of em).
    pub radical_vertical_gap: f32,
    /// Vertical gap between radicand top and radical rule in display style (fraction of em).
    pub radical_display_style_vertical_gap: f32,
    /// Thickness of the radical rule (fraction of em).
    pub radical_rule_thickness: f32,
    /// Extra ascender space above the radical rule (fraction of em).
    pub radical_extra_ascender: f32,
    /// Height of the bottom of the radical degree in proportion to radical height.
    pub radical_degree_bottom_raise_percent: f32,
    /// Extra horizontal kern before the radical degree (fraction of em).
    pub radical_kern_before_degree: f32,
    /// Negative horizontal kern after the radical degree (fraction of em).
    pub radical_kern_after_degree: f32,
}

impl Default for MathConstants {
    fn default() -> Self {
        Self {
            axis_height: 0.258,
            fraction_rule_thickness: 0.068,
            fraction_num_display_style_shift_up: 0.640,
            fraction_num_shift_up: 0.585,
            fraction_den_display_style_shift_down: 0.640,
            fraction_den_shift_down: 0.585,
            fraction_num_display_style_gap_min: 0.150,
            fraction_num_gap_min: 0.068,
            fraction_den_display_style_gap_min: 0.150,
            fraction_den_gap_min: 0.068,
            script_percent_scale_down: 0.70,
            script_script_percent_scale_down: 0.55,
            superscript_shift_up: 0.360,
            superscript_shift_up_cramped: 0.252,
            subscript_shift_down: 0.210,
            sub_superscript_gap_min: 0.150,
            radical_vertical_gap: 0.085,
            radical_display_style_vertical_gap: 0.170,
            radical_rule_thickness: 0.068,
            radical_extra_ascender: 0.078,
            radical_degree_bottom_raise_percent: 0.55,
            radical_kern_before_degree: 0.065,
            radical_kern_after_degree: -0.335,
        }
    }
}

/// Part record for vertical assembly of large radical signs.
#[derive(Clone, Copy, Debug)]
pub struct RadicalPart {
    pub glyph_id: u16,
    pub start_connector: u16,
    pub end_connector: u16,
    pub full_advance: u16,
    pub is_extender: bool,
}

/// Assembly recipe for vertically growing radical signs from OpenType MATH table.
#[derive(Clone, Debug)]
pub struct RadicalAssembly {
    pub min_connector_overlap: u16,
    pub parts: Vec<RadicalPart>,
}

/// Precomputed radical variants and assembly info for `U+221A`.
#[derive(Clone, Debug)]
pub struct RadicalInfo {
    /// Ascending list of `(glyph_id, advance_in_font_units)`.
    pub variants: Vec<(u16, u16)>,
    /// Assembly recipe when expressions exceed the tallest variant.
    pub assembly: Option<RadicalAssembly>,
}

/// Which face to draw a character in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathStyle {
    Text,         // Times New Roman
    Function,     // Times New Roman (sin, cos, lim...)
    Variable,     // Times New Roman Italic (x, y, z...)
    LcGreek,      // Lowercase Greek Italic (α, β, γ...)
    UcGreek,      // Uppercase Greek Regular (Γ, Δ, Ω...)
    Symbol,       // Symbol (+, -, =, ±...)
    VectorMatrix, // Times New Roman Bold
    Number,       // Times New Roman (0-9)
    ExtraMath,    // MT Extra (∫, ∑, √, large brackets, ...)
}

pub type Face = MathStyle;

/// One drawing command of a glyph outline, in absolute pixel coordinates (y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    /// Start a new contour.
    Move(f32, f32),
    /// Straight line to a point.
    Line(f32, f32),
    /// Quadratic Bézier: control point, end point.
    Quad(f32, f32, f32, f32),
    /// Cubic Bézier: two control points, end point.
    Cubic(f32, f32, f32, f32, f32, f32),
    /// Close the current contour.
    Close,
}

impl MathStyle {
    pub fn auto_detect(c: char) -> Self {
        match c {
            '0'..='9' => MathStyle::Number,
            'a'..='z' | 'A'..='Z' => MathStyle::Variable,
            '\u{03B1}'..='\u{03C9}' => MathStyle::LcGreek,
            '\u{0391}'..='\u{03A9}' => MathStyle::UcGreek,
            '∫' | '∬' | '∭' | '∑' | '∏' | '√' => MathStyle::ExtraMath,
            '+' | '-' | '=' | '<' | '>' | '±' | '×' | '÷' | '≤' | '≥' | '≠' | '≈' | '∈' | '∉' => {
                MathStyle::Symbol
            }
            _ => MathStyle::Text,
        }
    }
}

/// The font set.
pub struct Fonts {
    regular: FontRef<'static>,
    italic: FontRef<'static>,
    bold: FontRef<'static>,
    math: FontRef<'static>,
    math_face: ttf_parser::Face<'static>,
    constants: MathConstants,
    radical_info: RadicalInfo,
    ssty_map: HashMap<u16, (u16, u16)>,
}

/// Access the lazily-loaded font set.
pub fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| {
        let load = |d: &'static [u8]| FontRef::try_from_slice(d).expect("bundled font is valid");
        let math_face = ttf_parser::Face::parse(FONT_MATH, 0).expect("bundled STIX Two Math font is valid");
        let upem = math_face.units_per_em() as f32;
        let constants = if let Some(math) = math_face.tables().math {
            if let Some(c) = math.constants {
                MathConstants {
                    axis_height: c.axis_height().value as f32 / upem,
                    fraction_rule_thickness: c.fraction_rule_thickness().value as f32 / upem,
                    fraction_num_display_style_shift_up: c.fraction_numerator_display_style_shift_up().value as f32 / upem,
                    fraction_num_shift_up: c.fraction_numerator_shift_up().value as f32 / upem,
                    fraction_den_display_style_shift_down: c.fraction_denominator_display_style_shift_down().value as f32 / upem,
                    fraction_den_shift_down: c.fraction_denominator_shift_down().value as f32 / upem,
                    fraction_num_display_style_gap_min: c.fraction_num_display_style_gap_min().value as f32 / upem,
                    fraction_num_gap_min: c.fraction_numerator_gap_min().value as f32 / upem,
                    fraction_den_display_style_gap_min: c.fraction_denom_display_style_gap_min().value as f32 / upem,
                    fraction_den_gap_min: c.fraction_denominator_gap_min().value as f32 / upem,
                    script_percent_scale_down: c.script_percent_scale_down() as f32 / 100.0,
                    script_script_percent_scale_down: c.script_script_percent_scale_down() as f32 / 100.0,
                    superscript_shift_up: c.superscript_shift_up().value as f32 / upem,
                    superscript_shift_up_cramped: c.superscript_shift_up_cramped().value as f32 / upem,
                    subscript_shift_down: c.subscript_shift_down().value as f32 / upem,
                    sub_superscript_gap_min: c.sub_superscript_gap_min().value as f32 / upem,
                    radical_vertical_gap: c.radical_vertical_gap().value as f32 / upem,
                    radical_display_style_vertical_gap: c.radical_display_style_vertical_gap().value as f32 / upem,
                    radical_rule_thickness: c.radical_rule_thickness().value as f32 / upem,
                    radical_extra_ascender: c.radical_extra_ascender().value as f32 / upem,
                    radical_degree_bottom_raise_percent: c.radical_degree_bottom_raise_percent() as f32 / 100.0,
                    radical_kern_before_degree: c.radical_kern_before_degree().value as f32 / upem,
                    radical_kern_after_degree: c.radical_kern_after_degree().value as f32 / upem,
                }
            } else {
                MathConstants::default()
            }
        } else {
            MathConstants::default()
        };

        // Extract radical variants & assembly from OpenType MATH Variants table
        let gid_radical = math_face.glyph_index('\u{221A}');
        let mut variants_list = Vec::new();
        let mut assembly_opt = None;
        if let Some(gid) = gid_radical {
            if let Some(variants) = math_face.tables().math.and_then(|m| m.variants) {
                if let Some(construction) = variants.vertical_constructions.get(gid) {
                    for v in construction.variants {
                        variants_list.push((v.variant_glyph.0, v.advance_measurement));
                    }
                    if let Some(ga) = construction.assembly {
                        let mut parts = Vec::new();
                        for p in ga.parts {
                            parts.push(RadicalPart {
                                glyph_id: p.glyph_id.0,
                                start_connector: p.start_connector_length,
                                end_connector: p.end_connector_length,
                                full_advance: p.full_advance,
                                is_extender: p.part_flags.extender(),
                            });
                        }
                        assembly_opt = Some(RadicalAssembly {
                            min_connector_overlap: variants.min_connector_overlap,
                            parts,
                        });
                    }
                }
            }
        }
/// Extract optical size alternate glyph IDs for script levels from OpenType GSUB `ssty` feature.
///
/// Maps base `GlyphId (u16)` -> `(script_level_1_glyph, script_level_2_glyph)`.
fn parse_ssty_map(math_face: &ttf_parser::Face<'static>) -> HashMap<u16, (u16, u16)> {
    let mut ssty_map = HashMap::new();
    if let Some(gsub) = math_face.tables().gsub {
        if let Some(feature) = gsub.features.find(ttf_parser::Tag::from_bytes(b"ssty")) {
            for lookup_idx in feature.lookup_indices {
                if let Some(lookup) = gsub.lookups.get(lookup_idx) {
                    for st_idx in 0..lookup.subtables.len() {
                        if let Some(ttf_parser::gsub::SubstitutionSubtable::Alternate(alt_sub)) =
                            lookup.subtables.get::<ttf_parser::gsub::SubstitutionSubtable>(st_idx)
                        {
                            match alt_sub.coverage {
                                ttf_parser::opentype_layout::Coverage::Format1 { glyphs } => {
                                    for (cov_idx, gid) in glyphs.into_iter().enumerate() {
                                        if let Some(set) = alt_sub.alternate_sets.get(cov_idx as u16) {
                                            let s1 = set.alternates.get(0).map(|g| g.0).unwrap_or(gid.0);
                                            let s2 = set.alternates.get(1).map(|g| g.0).unwrap_or(s1);
                                            ssty_map.insert(gid.0, (s1, s2));
                                        }
                                    }
                                }
                                ttf_parser::opentype_layout::Coverage::Format2 { records } => {
                                    for rec in records {
                                        for offset in 0..=(rec.end.0 - rec.start.0) {
                                            let gid = rec.start.0 + offset;
                                            let cov_idx = rec.value + offset;
                                            if let Some(set) = alt_sub.alternate_sets.get(cov_idx) {
                                                let s1 = set.alternates.get(0).map(|g| g.0).unwrap_or(gid);
                                                let s2 = set.alternates.get(1).map(|g| g.0).unwrap_or(s1);
                                                ssty_map.insert(gid, (s1, s2));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ssty_map
}

        let radical_info = RadicalInfo {
            variants: variants_list,
            assembly: assembly_opt,
        };
        let ssty_map = parse_ssty_map(&math_face);

        Fonts {
            regular: load(FONT_REGULAR),
            italic: load(FONT_ITALIC),
            bold: load(FONT_BOLD),
            math: load(FONT_MATH),
            math_face,
            constants,
            radical_info,
            ssty_map,
        }
    })
}

fn outline_to_segments(outline: &ab_glyph::Outline, upem: f32, em_px: f32, x: f32, y: f32) -> Vec<Seg> {
    use ab_glyph::OutlineCurve as C;
    let k = em_px / upem;
    let pt = |p: ab_glyph::Point| (x + p.x * k, y - p.y * k);
    let mut segs = Vec::with_capacity(outline.curves.len() + 8);
    let mut pen: Option<(f32, f32)> = None;
    for c in &outline.curves {
        let start = match c {
            C::Line(a, _) | C::Quad(a, _, _) | C::Cubic(a, _, _, _) => pt(*a),
        };
        if pen != Some(start) {
            if pen.is_some() {
                segs.push(Seg::Close);
            }
            segs.push(Seg::Move(start.0, start.1));
        }
        pen = Some(match c {
            C::Line(_, b) => {
                let b = pt(*b);
                segs.push(Seg::Line(b.0, b.1));
                b
            }
            C::Quad(_, b, e) => {
                let (b, e) = (pt(*b), pt(*e));
                segs.push(Seg::Quad(b.0, b.1, e.0, e.1));
                e
            }
            C::Cubic(_, b, c2, e) => {
                let (b, c2, e) = (pt(*b), pt(*c2), pt(*e));
                segs.push(Seg::Cubic(b.0, b.1, c2.0, c2.1, e.0, e.1));
                e
            }
        });
    }
    if pen.is_some() {
        segs.push(Seg::Close);
    }
    segs
}

fn segments_to_polys(segs: &[Seg], em_x: f32, em_y: f32, x: f32, y: f32) -> Vec<Vec<(f32, f32)>> {
    let k = if em_y != 0.0 { em_x / em_y } else { 1.0 };
    let map = |px: f32, py: f32| (x + px * k, y + py);
    const STEPS: usize = 12;

    let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut cur: Vec<(f32, f32)> = Vec::new();
    let mut pen = (0.0f32, 0.0f32);
    for seg in segs {
        match seg {
            Seg::Move(a, b) => {
                if cur.len() >= 3 {
                    polys.push(std::mem::take(&mut cur));
                }
                cur.clear();
                pen = (*a, *b);
                cur.push(map(*a, *b));
            }
            Seg::Line(a, b) => {
                pen = (*a, *b);
                cur.push(map(*a, *b));
            }
            Seg::Quad(cx, cy, ex, ey) => {
                let (sx, sy) = pen;
                for i in 1..=STEPS {
                    let t = i as f32 / STEPS as f32;
                    let u = 1.0 - t;
                    cur.push(map(u * u * sx + 2.0 * u * t * cx + t * t * ex, u * u * sy + 2.0 * u * t * cy + t * t * ey));
                }
                pen = (*ex, *ey);
            }
            Seg::Cubic(c1x, c1y, c2x, c2y, ex, ey) => {
                let (sx, sy) = pen;
                for i in 1..=STEPS {
                    let t = i as f32 / STEPS as f32;
                    let u = 1.0 - t;
                    let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    cur.push(map(b0 * sx + b1 * c1x + b2 * c2x + b3 * ex, b0 * sy + b1 * c1y + b2 * c2y + b3 * ey));
                }
                pen = (*ex, *ey);
            }
            Seg::Close => {
                if cur.len() >= 3 {
                    polys.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
        }
    }
    if cur.len() >= 3 {
        polys.push(cur);
    }
    polys
}

impl Fonts {
    /// Access OpenType `MATH` table constants from `STIXTwoMath-Regular.ttf`.
    pub fn constants(&self) -> &MathConstants {
        &self.constants
    }

    /// Access radical variants and assembly information for `\u{221A}`.
    pub fn radical_info(&self) -> &RadicalInfo {
        &self.radical_info
    }

    /// Access optical size alternates map extracted from OpenType GSUB `ssty` feature.
    pub fn ssty_map(&self) -> &HashMap<u16, (u16, u16)> {
        &self.ssty_map
    }

    /// Look up optical size alternate glyph index in STIX Two Math font for script styles.
    ///
    /// - `level == 1`: Script style (`.ssty`)
    /// - `level >= 2`: ScriptScript style (`.ssty2`)
    /// - `level == 0`: returns the original glyph ID unchanged.
    pub fn ssty_glyph(&self, gid: u16, level: u8) -> u16 {
        match level {
            1 => self.ssty_map.get(&gid).map(|(s1, _)| *s1).unwrap_or(gid),
            2.. => self.ssty_map.get(&gid).map(|(_, s2)| *s2).unwrap_or(gid),
            _ => gid,
        }
    }

    /// Convert character and style to the appropriate Unicode Mathematical character in STIX Two Math.
    pub fn to_math_char(style: MathStyle, ch: char) -> char {
        if style == MathStyle::Variable {
            if ch == 'h' {
                '\u{210E}' // Planck constant h
            } else if ch.is_ascii_lowercase() {
                char::from_u32(0x1D44E + (ch as u32 - 'a' as u32)).unwrap_or(ch)
            } else if ch.is_ascii_uppercase() {
                char::from_u32(0x1D434 + (ch as u32 - 'A' as u32)).unwrap_or(ch)
            } else {
                ch
            }
        } else if style == MathStyle::LcGreek && ('\u{03B1}'..='\u{03C9}').contains(&ch) {
            char::from_u32(ch as u32 + 0x1D34B).unwrap_or(ch)
        } else {
            ch
        }
    }

    /// Look up glyph index in STIX Two Math font.
    pub fn glyph_id_for_math(&self, style: MathStyle, ch: char) -> Option<ttf_parser::GlyphId> {
        self.glyph_id_for_math_styled(style, ch, 0)
    }

    /// Look up glyph index in STIX Two Math font, optionally substituting optical size variant.
    pub fn glyph_id_for_math_styled(&self, style: MathStyle, ch: char, ssty: u8) -> Option<ttf_parser::GlyphId> {
        let math_ch = Self::to_math_char(style, ch);
        let base_gid = self.math_face.glyph_index(math_ch).or_else(|| self.math_face.glyph_index(ch))?;
        if ssty > 0 {
            Some(ttf_parser::GlyphId(self.ssty_glyph(base_gid.0, ssty)))
        } else {
            Some(base_gid)
        }
    }

    /// Retrieve the italic correction (in pixels) for `ch` under `style` at `em_px`.
    pub fn italic_correction(&self, style: MathStyle, ch: char, em_px: f32) -> f32 {
        self.italic_correction_styled(style, ch, em_px, 0)
    }

    /// Retrieve the italic correction (in pixels) for `ch` under `style` at `em_px`,
    /// accounting for script optical alternate glyphs if `ssty > 0`.
    pub fn italic_correction_styled(&self, style: MathStyle, ch: char, em_px: f32, ssty: u8) -> f32 {
        let base_gid = self.glyph_id_for_math(style, ch);
        let eff_gid = if ssty > 0 {
            base_gid.map(|g| ttf_parser::GlyphId(self.ssty_glyph(g.0, ssty)))
        } else {
            base_gid
        };

        if let Some(math) = self.math_face.tables().math {
            if let Some(gi) = math.glyph_info {
                if let Some(ics) = gi.italic_corrections {
                    if let Some(gid) = eff_gid {
                        if let Some(v) = ics.get(gid) {
                            return (v.value as f32 / self.math_face.units_per_em() as f32) * em_px;
                        }
                    }
                    if let Some(gid) = base_gid {
                        if let Some(v) = ics.get(gid) {
                            return (v.value as f32 / self.math_face.units_per_em() as f32) * em_px;
                        }
                    }
                }
            }
        }
        0.0
    }

    /// Retrieve the horizontal top accent attachment position (in pixels from left of glyph origin)
    /// for `ch` under `style` at `em_px`.
    pub fn top_accent_attachment(&self, style: MathStyle, ch: char, em_px: f32) -> Option<f32> {
        if let Some(gid) = self.glyph_id_for_math(style, ch) {
            if let Some(math) = self.math_face.tables().math {
                if let Some(gi) = math.glyph_info {
                    if let Some(tas) = gi.top_accent_attachments {
                        if let Some(v) = tas.get(gid) {
                            return Some((v.value as f32 / self.math_face.units_per_em() as f32) * em_px);
                        }
                    }
                }
            }
        }
        None
    }

    /// Calculate the horizontal math kerning adjustment between a base glyph and a script glyph.
    ///
    /// For superscript (`is_sub == false`), examines base's `top_right` kern and script's `bottom_left` kern.
    /// For subscript (`is_sub == true`), examines base's `bottom_right` kern and script's `top_left` kern.
    pub fn math_kern(
        &self,
        base_style: MathStyle,
        base_ch: char,
        script_style: MathStyle,
        script_ch: char,
        is_sub: bool,
        script_y: f32,
        em_px: f32,
    ) -> f32 {
        self.math_kern_styled(base_style, base_ch, script_style, script_ch, is_sub, script_y, em_px, 0)
    }

    pub fn math_kern_styled(
        &self,
        base_style: MathStyle,
        base_ch: char,
        script_style: MathStyle,
        script_ch: char,
        is_sub: bool,
        script_y: f32,
        em_px: f32,
        ssty: u8,
    ) -> f32 {
        let upem = self.math_face.units_per_em() as f32;
        let mut total_kern_units: i32 = 0;

        let eval_kern = |kern: &ttf_parser::math::Kern, height: i16| -> i16 {
            let n = kern.count();
            let mut idx = 0;
            while idx < n {
                if let Some(h) = kern.height(idx) {
                    if height <= h.value {
                        return kern.kern(idx).map_or(0, |k| k.value);
                    }
                }
                idx += 1;
            }
            kern.kern(n).map_or(0, |k| k.value)
        };

        let b_gid_opt = self.glyph_id_for_math(base_style, base_ch);
        let s_gid_opt = self.glyph_id_for_math_styled(script_style, script_ch, ssty)
            .or_else(|| self.glyph_id_for_math(script_style, script_ch));
        if let (Some(b_gid), Some(s_gid)) = (b_gid_opt, s_gid_opt) {
            if let Some(math) = self.math_face.tables().math {
                if let Some(gi) = math.glyph_info {
                    if let Some(kern_info) = gi.kern_infos {
                        let h_units = (-(script_y / em_px) * upem).round() as i16;

                        if let Some(b_record) = kern_info.get(b_gid) {
                            let b_kern = if is_sub {
                                b_record.bottom_right.as_ref()
                            } else {
                                b_record.top_right.as_ref()
                            };
                            if let Some(k) = b_kern {
                                total_kern_units += eval_kern(k, h_units) as i32;
                            }
                        }

                        if let Some(s_record) = kern_info.get(s_gid) {
                            let s_kern = if is_sub {
                                s_record.top_left.as_ref()
                            } else {
                                s_record.bottom_left.as_ref()
                            };
                            if let Some(k) = s_kern {
                                total_kern_units += eval_kern(k, h_units) as i32;
                            }
                        }
                    }
                }
            }
        }

        (total_kern_units as f32 / upem) * em_px
    }

    fn pick(&self, style: MathStyle, ch: char) -> (&FontRef<'static>, GlyphId) {
        self.pick_styled(style, ch, 0)
    }

    fn pick_styled(&self, style: MathStyle, ch: char, ssty: u8) -> (&FontRef<'static>, GlyphId) {
        if ssty > 0 {
            if let Some(base_gid) = self.glyph_id_for_math(style, ch) {
                let alt_gid = self.ssty_glyph(base_gid.0, ssty);
                if alt_gid != base_gid.0 {
                    return (&self.math, GlyphId(alt_gid));
                }
            }
        }

        let mut ch = ch;
        if style == MathStyle::LcGreek && ('\u{03B1}'..='\u{03C9}').contains(&ch) {
            ch = std::char::from_u32(ch as u32 + 0x1D34B).unwrap_or(ch);
        }

        let primary = match style {
            MathStyle::Text | MathStyle::Function | MathStyle::Number => &self.regular,
            MathStyle::Variable => &self.italic,
            MathStyle::VectorMatrix => &self.bold,
            MathStyle::LcGreek | MathStyle::UcGreek => &self.math,
            MathStyle::Symbol | MathStyle::ExtraMath => &self.math,
        };

        let fallback_chain = match style {
            MathStyle::LcGreek => [primary, &self.math, &self.regular],
            MathStyle::Symbol | MathStyle::ExtraMath => [primary, &self.regular, &self.italic],
            _ => [primary, &self.math, &self.regular],
        };

        for f in fallback_chain {
            let id = f.glyph_id(ch);
            if id.0 != 0 {
                return (f, id);
            }
        }

        (primary, primary.glyph_id(ch))
    }

    fn scale_for(font: &FontRef<'static>, em_px: f32) -> PxScale {
        let upem = font.units_per_em().unwrap_or(1000.0);
        PxScale::from(em_px * font.height_unscaled() / upem)
    }

    pub fn advance(&self, style: MathStyle, ch: char, em_px: f32) -> f32 {
        self.advance_styled(style, ch, em_px, 0)
    }

    pub fn advance_styled(&self, style: MathStyle, ch: char, em_px: f32, ssty: u8) -> f32 {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let adv = f.h_advance_unscaled(id) * em_px / f.units_per_em().unwrap_or(1000.0);
        adv
    }

    pub fn ink(&self, style: MathStyle, ch: char, em_px: f32) -> (f32, f32) {
        self.ink_styled(style, ch, em_px, 0)
    }

    pub fn ink_styled(&self, style: MathStyle, ch: char, em_px: f32, ssty: u8) -> (f32, f32) {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let g = id.with_scale_and_position(Self::scale_for(f, em_px), ab_glyph::point(0.0, 0.0));
        match f.outline_glyph(g) {
            Some(o) => {
                let b = o.px_bounds();
                ((-b.min.y).max(0.0), b.max.y.max(0.0))
            }
            None => (0.0, 0.0),
        }
    }

    pub fn ink_x(&self, style: MathStyle, ch: char, em_px: f32) -> (f32, f32) {
        self.ink_x_styled(style, ch, em_px, 0)
    }

    pub fn ink_x_styled(&self, style: MathStyle, ch: char, em_px: f32, ssty: u8) -> (f32, f32) {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let g = id.with_scale_and_position(Self::scale_for(f, em_px), ab_glyph::point(0.0, 0.0));
        match f.outline_glyph(g) {
            Some(o) => {
                let b = o.px_bounds();
                (b.min.x, b.max.x)
            }
            None => (0.0, 0.0),
        }
    }

    pub fn draw(&self, style: MathStyle, ch: char, em_px: f32, x: f32, y: f32, canvas: &mut Canvas) {
        self.draw_styled(style, ch, em_px, x, y, canvas, 0);
    }

    pub fn draw_styled(&self, style: MathStyle, ch: char, em_px: f32, x: f32, y: f32, canvas: &mut Canvas, ssty: u8) {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let g = id.with_scale_and_position(Self::scale_for(f, em_px), ab_glyph::point(x, y));
        if let Some(o) = f.outline_glyph(g) {
            let b = o.px_bounds();
            let (bx, by) = (b.min.x.floor() as i64, b.min.y.floor() as i64);
            o.draw(|gx, gy, c| canvas.add(bx + gx as i64, by + gy as i64, c));
        }
    }

    /// The outline of `ch` as drawing commands in pixels (y pointing down), with its
    /// baseline origin at `(x, y)` and `em_px` pixels per em. `None` when the glyph has
    /// no outline (e.g. a space).
    pub fn glyph_segments(&self, style: MathStyle, ch: char, em_px: f32, x: f32, y: f32) -> Option<Vec<Seg>> {
        self.glyph_segments_styled(style, ch, em_px, x, y, 0)
    }

    pub fn glyph_segments_styled(&self, style: MathStyle, ch: char, em_px: f32, x: f32, y: f32, ssty: u8) -> Option<Vec<Seg>> {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let outline = f.outline(id)?;
        let upem = f.units_per_em().unwrap_or(1000.0);
        Some(outline_to_segments(&outline, upem, em_px, x, y))
    }

    /// The outline of `ch` flattened to polygons (one per contour), drawn with
    /// independent horizontal and vertical scales: `em_x` / `em_y` pixels per em.
    /// The baseline origin sits at `(x, y)`; pixels grow downwards.
    pub fn glyph_polys(&self, style: MathStyle, ch: char, em_x: f32, em_y: f32, x: f32, y: f32) -> Vec<Vec<(f32, f32)>> {
        self.glyph_polys_styled(style, ch, em_x, em_y, x, y, 0)
    }

    pub fn glyph_polys_styled(&self, style: MathStyle, ch: char, em_x: f32, em_y: f32, x: f32, y: f32, ssty: u8) -> Vec<Vec<(f32, f32)>> {
        let Some(segs) = self.glyph_segments_styled(style, ch, em_y, 0.0, 0.0, ssty) else {
            return Vec::new();
        };
        segments_to_polys(&segs, em_x, em_y, x, y)
    }

    /// Extract polygon contours of a glyph directly from `STIXTwoMath` by its raw `glyph_id`.
    pub fn math_glyph_polys(&self, gid: u16, em_x: f32, em_y: f32, x: f32, y: f32) -> Vec<Vec<(f32, f32)>> {
        let f = &self.math;
        let id = ab_glyph::GlyphId(gid);
        let Some(outline) = f.outline(id) else {
            return Vec::new();
        };
        let upem = f.units_per_em().unwrap_or(1000.0);
        let segs = outline_to_segments(&outline, upem, em_y, 0.0, 0.0);
        segments_to_polys(&segs, em_x, em_y, x, y)
    }

    /// Bounding box `(x_min, y_min, x_max, y_max)` in font design units for a glyph in `STIXTwoMath`.
    pub fn math_glyph_bbox(&self, gid: u16) -> Option<(i16, i16, i16, i16)> {
        self.math_face
            .glyph_bounding_box(ttf_parser::GlyphId(gid))
            .map(|r| (r.x_min, r.y_min, r.x_max, r.y_max))
    }

    pub fn has_glyph(&self, ch: char) -> bool {
        self.pick(MathStyle::Text, ch).1 .0 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_constants_and_glyph_info() {
        let f = fonts();
        let c = f.constants();
        assert_eq!((c.axis_height * 1000.0).round() as i32, 258);
        assert_eq!((c.fraction_rule_thickness * 1000.0).round() as i32, 68);
        assert_eq!((c.script_percent_scale_down * 100.0).round() as i32, 70);

        // Radical constants
        assert_eq!((c.radical_degree_bottom_raise_percent * 100.0).round() as i32, 55);
        assert_eq!((c.radical_kern_before_degree * 1000.0).round() as i32, 65);
        assert_eq!((c.radical_kern_after_degree * 1000.0).round() as i32, -335);

        // Radical variants and assembly
        let r_info = f.radical_info();
        assert_eq!(r_info.variants.len(), 4);
        assert!(r_info.assembly.is_some());
        let ass = r_info.assembly.as_ref().unwrap();
        assert_eq!(ass.parts.len(), 3);
        assert_eq!(ass.min_connector_overlap, 100);

        // Verify italic correction on math italic f
        let ic_f = f.italic_correction(MathStyle::Variable, 'f', 100.0);
        assert!(ic_f > 0.0);

        // Verify top accent attachment on V
        let ta_v = f.top_accent_attachment(MathStyle::Variable, 'V', 100.0);
        assert!(ta_v.is_some());

        // Verify math kerning on P with subscript
        let mk_p = f.math_kern(MathStyle::Variable, 'P', MathStyle::Variable, 'i', true, 5.0, 100.0);
        assert!(mk_p < 0.0, "Subscript under P should be kerned negatively, got {}", mk_p);

        // Verify GSUB ssty feature (Phase 6)
        assert_eq!(f.ssty_map().len(), 529, "Expected 529 ssty alternate mappings in STIXTwoMath");

        // Verify optical size substitution for math italic x
        let x_gid = f.glyph_id_for_math(MathStyle::Variable, 'x').expect("math italic x must exist");
        let x_ssty1 = f.ssty_glyph(x_gid.0, 1);
        let x_ssty2 = f.ssty_glyph(x_gid.0, 2);
        assert_ne!(x_ssty1, x_gid.0, "ssty level 1 alternate should differ from base");
        assert_ne!(x_ssty2, x_gid.0, "ssty level 2 alternate should differ from base");
        assert_ne!(x_ssty1, x_ssty2, "ssty level 1 and 2 should differ");

        // Verify optical advance: ssty glyph is designed wider with heavier stems
        let adv_norm = f.advance(MathStyle::Variable, 'x', 100.0);
        let adv_ssty1 = f.advance_styled(MathStyle::Variable, 'x', 100.0, 1);
        let adv_ssty2 = f.advance_styled(MathStyle::Variable, 'x', 100.0, 2);
        assert!(adv_ssty1 > adv_norm, "ssty level 1 advance ({}) must be wider than base ({})", adv_ssty1, adv_norm);
        assert!(adv_ssty2 > adv_ssty1, "ssty level 2 advance ({}) must be wider than level 1 ({})", adv_ssty2, adv_ssty1);

        // Verify outline segments are successfully produced for ssty glyph
        let segs = f.glyph_segments_styled(MathStyle::Variable, 'x', 100.0, 0.0, 0.0, 1);
        assert!(segs.is_some() && !segs.unwrap().is_empty(), "ssty glyph must produce valid outline segments");
    }
}
