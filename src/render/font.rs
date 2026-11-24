//! Embedded fonts and glyph helpers used to draw equation previews.

use std::collections::HashMap;
use std::sync::OnceLock;

use read_fonts::tables::math::{MathKern, MathKernCorner};
use read_fonts::TableProvider;
use skrifa::charmap::Charmap;
use skrifa::metrics::GlyphMetrics;
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, GlyphId};

use super::canvas::Canvas;

mod geometry;
mod metadata;
mod selection;

use geometry::{outline_to_segments, rasterize_segments, segments_bounds, segments_to_polys};

static FONT_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Regular.ttf");
static FONT_ITALIC: &[u8] =
    include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Italic.ttf");
static FONT_BOLD: &[u8] =
    include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Bold.ttf");
static FONT_MATH: &[u8] =
    include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoMath-Regular.ttf");

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
    constants: MathConstants,
    radical_info: RadicalInfo,
    ssty_map: HashMap<u16, (u16, u16)>,
}

fn load_bundled_font(data: &'static [u8]) -> FontRef<'static> {
    FontRef::new(data).expect("bundled font is valid")
}

/// Access the lazily-loaded font set.
pub fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| {
        let math_face = load_bundled_font(FONT_MATH);
        let upem = Fonts::units_per_em(&math_face);
        let constants = if let Ok(math) = math_face.math() {
            if let Ok(c) = math.math_constants() {
                MathConstants {
                    axis_height: c.axis_height().value().to_i16() as f32 / upem,
                    fraction_rule_thickness: c.fraction_rule_thickness().value().to_i16() as f32
                        / upem,
                    fraction_num_display_style_shift_up: c
                        .fraction_numerator_display_style_shift_up()
                        .value()
                        .to_i16() as f32
                        / upem,
                    fraction_num_shift_up: c.fraction_numerator_shift_up().value().to_i16() as f32
                        / upem,
                    fraction_den_display_style_shift_down: c
                        .fraction_denominator_display_style_shift_down()
                        .value()
                        .to_i16() as f32
                        / upem,
                    fraction_den_shift_down: c.fraction_denominator_shift_down().value().to_i16()
                        as f32
                        / upem,
                    fraction_num_display_style_gap_min: c
                        .fraction_num_display_style_gap_min()
                        .value()
                        .to_i16() as f32
                        / upem,
                    fraction_num_gap_min: c.fraction_numerator_gap_min().value().to_i16() as f32
                        / upem,
                    fraction_den_display_style_gap_min: c
                        .fraction_denom_display_style_gap_min()
                        .value()
                        .to_i16() as f32
                        / upem,
                    fraction_den_gap_min: c.fraction_denominator_gap_min().value().to_i16() as f32
                        / upem,
                    script_percent_scale_down: c.script_percent_scale_down() as f32 / 100.0,
                    script_script_percent_scale_down: c.script_script_percent_scale_down() as f32
                        / 100.0,
                    superscript_shift_up: c.superscript_shift_up().value().to_i16() as f32 / upem,
                    superscript_shift_up_cramped: c.superscript_shift_up_cramped().value().to_i16()
                        as f32
                        / upem,
                    subscript_shift_down: c.subscript_shift_down().value().to_i16() as f32 / upem,
                    sub_superscript_gap_min: c.sub_superscript_gap_min().value().to_i16() as f32
                        / upem,
                    radical_vertical_gap: c.radical_vertical_gap().value().to_i16() as f32 / upem,
                    radical_display_style_vertical_gap: c
                        .radical_display_style_vertical_gap()
                        .value()
                        .to_i16() as f32
                        / upem,
                    radical_rule_thickness: c.radical_rule_thickness().value().to_i16() as f32
                        / upem,
                    radical_extra_ascender: c.radical_extra_ascender().value().to_i16() as f32
                        / upem,
                    radical_degree_bottom_raise_percent: c.radical_degree_bottom_raise_percent()
                        as f32
                        / 100.0,
                    radical_kern_before_degree: c.radical_kern_before_degree().value().to_i16()
                        as f32
                        / upem,
                    radical_kern_after_degree: c.radical_kern_after_degree().value().to_i16()
                        as f32
                        / upem,
                }
            } else {
                MathConstants::default()
            }
        } else {
            MathConstants::default()
        };

        let radical_info = metadata::radical_info(&math_face);
        let ssty_map = metadata::ssty_map(&math_face);

        Fonts {
            regular: load_bundled_font(FONT_REGULAR),
            italic: load_bundled_font(FONT_ITALIC),
            bold: load_bundled_font(FONT_BOLD),
            math: math_face,
            constants,
            radical_info,
            ssty_map,
        }
    })
}

impl Fonts {
    fn units_per_em(font: &FontRef<'static>) -> f32 {
        font.head()
            .map(|h| h.units_per_em() as f32)
            .unwrap_or(1000.0)
    }

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
    pub fn glyph_id_for_math(&self, style: MathStyle, ch: char) -> Option<u16> {
        self.glyph_id_for_math_styled(style, ch, 0)
    }

    /// Look up glyph index in STIX Two Math font, optionally substituting optical size variant.
    pub fn glyph_id_for_math_styled(&self, style: MathStyle, ch: char, ssty: u8) -> Option<u16> {
        let math_ch = Self::to_math_char(style, ch);
        let charmap = Charmap::new(&self.math);
        let base_gid = charmap
            .map(math_ch)
            .filter(|&g| g != GlyphId::NOTDEF)
            .or_else(|| charmap.map(ch).filter(|&g| g != GlyphId::NOTDEF))?;
        let base_u16 = u16::try_from(base_gid.to_u32()).ok()?;
        if ssty > 0 {
            Some(self.ssty_glyph(base_u16, ssty))
        } else {
            Some(base_u16)
        }
    }

    /// Retrieve the italic correction (in pixels) for `ch` under `style` at `em_px`.
    pub fn italic_correction(&self, style: MathStyle, ch: char, em_px: f32) -> f32 {
        self.italic_correction_styled(style, ch, em_px, 0)
    }

    /// Retrieve the italic correction (in pixels) for `ch` under `style` at `em_px`,
    /// accounting for script optical alternate glyphs if `ssty > 0`.
    pub fn italic_correction_styled(
        &self,
        style: MathStyle,
        ch: char,
        em_px: f32,
        ssty: u8,
    ) -> f32 {
        let base_gid = self.glyph_id_for_math(style, ch);
        let eff_gid = if ssty > 0 {
            base_gid.map(|g| self.ssty_glyph(g, ssty))
        } else {
            base_gid
        };

        let upem = Self::units_per_em(&self.math);
        if let Ok(math) = self.math.math() {
            if let Ok(gi) = math.math_glyph_info() {
                if let Some(Ok(ics)) = gi.math_italics_correction_info() {
                    if let Some(gid) = eff_gid {
                        if let Some(v) = ics.correction(GlyphId::from(gid)) {
                            return (v as f32 / upem) * em_px;
                        }
                    }
                    if let Some(gid) = base_gid {
                        if let Some(v) = ics.correction(GlyphId::from(gid)) {
                            return (v as f32 / upem) * em_px;
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
        let gid = self.glyph_id_for_math(style, ch)?;
        let upem = Self::units_per_em(&self.math);
        let math = self.math.math().ok()?;
        let gi = math.math_glyph_info().ok()?;
        let tas = gi.math_top_accent_attachment()?.ok()?;
        let v = tas.attachment(GlyphId::from(gid))?;
        Some((v as f32 / upem) * em_px)
    }

    /// Calculate the horizontal math kerning adjustment between a base glyph and a script glyph.
    ///
    /// For superscript (`is_sub == false`), examines base's `top_right` kern and script's `bottom_left` kern.
    /// For subscript (`is_sub == true`), examines base's `bottom_right` kern and script's `top_left` kern.
    #[allow(clippy::too_many_arguments)]
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
        self.math_kern_styled(
            base_style,
            base_ch,
            script_style,
            script_ch,
            is_sub,
            script_y,
            em_px,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
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
        let upem = Self::units_per_em(&self.math);
        let mut total_kern_units: i32 = 0;

        let eval_kern = |kern: &MathKern<'_>, height: i16| -> i16 {
            let heights = kern.correction_height();
            let values = kern.kern_values();
            for (idx, h) in heights.iter().enumerate() {
                if height <= h.value().to_i16() {
                    return values.get(idx).map_or(0, |k| k.value().to_i16());
                }
            }
            values.get(heights.len()).map_or(0, |k| k.value().to_i16())
        };

        let b_gid_opt = self.glyph_id_for_math(base_style, base_ch);
        let s_gid_opt = self
            .glyph_id_for_math_styled(script_style, script_ch, ssty)
            .or_else(|| self.glyph_id_for_math(script_style, script_ch));
        if let (Some(b_gid), Some(s_gid)) = (b_gid_opt, s_gid_opt) {
            if let Ok(math) = self.math.math() {
                if let Ok(gi) = math.math_glyph_info() {
                    if let Some(Ok(kern_info)) = gi.math_kern_info() {
                        let h_units = (-(script_y / em_px) * upem).round() as i16;

                        let b_corner = if is_sub {
                            MathKernCorner::BottomRight
                        } else {
                            MathKernCorner::TopRight
                        };
                        if let Some(k) = kern_info.kern(GlyphId::from(b_gid), b_corner) {
                            total_kern_units += eval_kern(&k, h_units) as i32;
                        }

                        let s_corner = if is_sub {
                            MathKernCorner::TopLeft
                        } else {
                            MathKernCorner::BottomLeft
                        };
                        if let Some(k) = kern_info.kern(GlyphId::from(s_gid), s_corner) {
                            total_kern_units += eval_kern(&k, h_units) as i32;
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
                let alt_gid = self.ssty_glyph(base_gid, ssty);
                if alt_gid != base_gid {
                    return (&self.math, GlyphId::from(alt_gid));
                }
            }
        }

        selection::pick_glyph(
            style,
            ch,
            &self.regular,
            &self.italic,
            &self.bold,
            &self.math,
        )
    }

    fn glyph_bbox(font: &FontRef<'static>, id: GlyphId) -> Option<(i16, i16, i16, i16)> {
        if let (Ok(loca), Ok(glyf)) = (font.loca(None), font.glyf()) {
            let glyph = loca.get_glyf(id, &glyf).ok()??;
            let bbox = (glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max());
            return (bbox.0 < bbox.2 && bbox.1 < bbox.3).then_some(bbox);
        }
        let bbox = GlyphMetrics::new(font, Size::unscaled(), LocationRef::default()).bounds(id)?;
        let res = (
            bbox.x_min.round() as i16,
            bbox.y_min.round() as i16,
            bbox.x_max.round() as i16,
            bbox.y_max.round() as i16,
        );
        (res.0 < res.2 && res.1 < res.3).then_some(res)
    }

    fn glyph_px_bounds(
        font: &FontRef<'static>,
        id: GlyphId,
        em_px: f32,
        x: f32,
        y: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        let (x_min, y_min, x_max, y_max) = Self::glyph_bbox(font, id)?;
        let scale = em_px / Self::units_per_em(font);
        let (x_trunc, x_fract) = (x.trunc(), x.fract());
        let (y_trunc, y_fract) = (y.trunc(), y.fract());
        Some((
            (x_min as f32 * scale + x_fract).floor() + x_trunc,
            (y_max as f32 * -scale + y_fract).floor() + y_trunc,
            (x_max as f32 * scale + x_fract).ceil() + x_trunc,
            (y_min as f32 * -scale + y_fract).ceil() + y_trunc,
        ))
    }

    pub fn advance(&self, style: MathStyle, ch: char, em_px: f32) -> f32 {
        self.advance_styled(style, ch, em_px, 0)
    }

    pub fn advance_styled(&self, style: MathStyle, ch: char, em_px: f32, ssty: u8) -> f32 {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let advance_unscaled = GlyphMetrics::new(f, Size::unscaled(), LocationRef::default())
            .advance_width(id)
            .unwrap_or(0.0);
        advance_unscaled * em_px / Self::units_per_em(f)
    }

    pub fn ink(&self, style: MathStyle, ch: char, em_px: f32) -> (f32, f32) {
        self.ink_styled(style, ch, em_px, 0)
    }

    pub fn ink_styled(&self, style: MathStyle, ch: char, em_px: f32, ssty: u8) -> (f32, f32) {
        let (f, id) = self.pick_styled(style, ch, ssty);
        match Self::glyph_px_bounds(f, id, em_px, 0.0, 0.0) {
            Some((_, min_y, _, max_y)) => ((-min_y).max(0.0), max_y.max(0.0)),
            None => (0.0, 0.0),
        }
    }

    pub fn ink_x(&self, style: MathStyle, ch: char, em_px: f32) -> (f32, f32) {
        self.ink_x_styled(style, ch, em_px, 0)
    }

    pub fn ink_x_styled(&self, style: MathStyle, ch: char, em_px: f32, ssty: u8) -> (f32, f32) {
        let (f, id) = self.pick_styled(style, ch, ssty);
        match Self::glyph_px_bounds(f, id, em_px, 0.0, 0.0) {
            Some((min_x, _, max_x, _)) => (min_x, max_x),
            None => (0.0, 0.0),
        }
    }

    pub fn draw(
        &self,
        style: MathStyle,
        ch: char,
        em_px: f32,
        x: f32,
        y: f32,
        canvas: &mut Canvas,
    ) {
        self.draw_styled(style, ch, em_px, x, y, canvas, 0);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_styled(
        &self,
        style: MathStyle,
        ch: char,
        em_px: f32,
        x: f32,
        y: f32,
        canvas: &mut Canvas,
        ssty: u8,
    ) {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let upem = Self::units_per_em(f);
        let Some(segs) = outline_to_segments(f, id, upem, em_px, x, y) else {
            return;
        };
        let bounds = Self::glyph_px_bounds(f, id, em_px, x, y).or_else(|| segments_bounds(&segs));
        let Some((min_x, min_y, max_x, max_y)) = bounds else {
            return;
        };
        let (min_x, min_y, max_x, max_y) = if let Some(seg_b) = segments_bounds(&segs) {
            (
                min_x.min(seg_b.0),
                min_y.min(seg_b.1),
                max_x.max(seg_b.2),
                max_y.max(seg_b.3),
            )
        } else {
            (min_x, min_y, max_x, max_y)
        };
        rasterize_segments(&segs, min_x, min_y, max_x, max_y, canvas);
    }

    /// The outline of `ch` as drawing commands in pixels (y pointing down), with its
    /// baseline origin at `(x, y)` and `em_px` pixels per em. `None` when the glyph has
    /// no outline (e.g. a space).
    pub fn glyph_segments(
        &self,
        style: MathStyle,
        ch: char,
        em_px: f32,
        x: f32,
        y: f32,
    ) -> Option<Vec<Seg>> {
        self.glyph_segments_styled(style, ch, em_px, x, y, 0)
    }

    pub fn glyph_segments_styled(
        &self,
        style: MathStyle,
        ch: char,
        em_px: f32,
        x: f32,
        y: f32,
        ssty: u8,
    ) -> Option<Vec<Seg>> {
        let (f, id) = self.pick_styled(style, ch, ssty);
        let upem = Self::units_per_em(f);
        outline_to_segments(f, id, upem, em_px, x, y)
    }

    /// The outline of `ch` flattened to polygons (one per contour), drawn with
    /// independent horizontal and vertical scales: `em_x` / `em_y` pixels per em.
    /// The baseline origin sits at `(x, y)`; pixels grow downwards.
    pub fn glyph_polys(
        &self,
        style: MathStyle,
        ch: char,
        em_x: f32,
        em_y: f32,
        x: f32,
        y: f32,
    ) -> Vec<Vec<(f32, f32)>> {
        self.glyph_polys_styled(style, ch, em_x, em_y, x, y, 0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn glyph_polys_styled(
        &self,
        style: MathStyle,
        ch: char,
        em_x: f32,
        em_y: f32,
        x: f32,
        y: f32,
        ssty: u8,
    ) -> Vec<Vec<(f32, f32)>> {
        let Some(segs) = self.glyph_segments_styled(style, ch, em_y, 0.0, 0.0, ssty) else {
            return Vec::new();
        };
        segments_to_polys(&segs, em_x, em_y, x, y)
    }

    /// Extract polygon contours of a glyph directly from `STIXTwoMath` by its raw `glyph_id`.
    pub fn math_glyph_polys(
        &self,
        gid: u16,
        em_x: f32,
        em_y: f32,
        x: f32,
        y: f32,
    ) -> Vec<Vec<(f32, f32)>> {
        let f = &self.math;
        let id = GlyphId::from(gid);
        let upem = Self::units_per_em(f);
        let Some(segs) = outline_to_segments(f, id, upem, em_y, 0.0, 0.0) else {
            return Vec::new();
        };
        segments_to_polys(&segs, em_x, em_y, x, y)
    }

    /// Bounding box `(x_min, y_min, x_max, y_max)` in font design units for a glyph in `STIXTwoMath`.
    pub fn math_glyph_bbox(&self, gid: u16) -> Option<(i16, i16, i16, i16)> {
        Self::glyph_bbox(&self.math, GlyphId::from(gid))
    }

    pub fn has_glyph(&self, ch: char) -> bool {
        self.pick(MathStyle::Text, ch).1 != GlyphId::NOTDEF
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
        assert_eq!(
            (c.radical_degree_bottom_raise_percent * 100.0).round() as i32,
            55
        );
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
        let mk_p = f.math_kern(
            MathStyle::Variable,
            'P',
            MathStyle::Variable,
            'i',
            true,
            5.0,
            100.0,
        );
        assert!(
            mk_p < 0.0,
            "Subscript under P should be kerned negatively, got {}",
            mk_p
        );

        // Verify GSUB ssty feature (Phase 6)
        assert_eq!(
            f.ssty_map().len(),
            529,
            "Expected 529 ssty alternate mappings in STIXTwoMath"
        );

        // Verify optical size substitution for math italic x
        let x_gid = f
            .glyph_id_for_math(MathStyle::Variable, 'x')
            .expect("math italic x must exist");
        let x_ssty1 = f.ssty_glyph(x_gid, 1);
        let x_ssty2 = f.ssty_glyph(x_gid, 2);
        assert_ne!(
            x_ssty1, x_gid,
            "ssty level 1 alternate should differ from base"
        );
        assert_ne!(
            x_ssty2, x_gid,
            "ssty level 2 alternate should differ from base"
        );
        assert_ne!(x_ssty1, x_ssty2, "ssty level 1 and 2 should differ");

        // Verify optical advance: ssty glyph is designed wider with heavier stems
        let adv_norm = f.advance(MathStyle::Variable, 'x', 100.0);
        let adv_ssty1 = f.advance_styled(MathStyle::Variable, 'x', 100.0, 1);
        let adv_ssty2 = f.advance_styled(MathStyle::Variable, 'x', 100.0, 2);
        assert!(
            adv_ssty1 > adv_norm,
            "ssty level 1 advance ({}) must be wider than base ({})",
            adv_ssty1,
            adv_norm
        );
        assert!(
            adv_ssty2 > adv_ssty1,
            "ssty level 2 advance ({}) must be wider than level 1 ({})",
            adv_ssty2,
            adv_ssty1
        );

        // Verify outline segments are successfully produced for ssty glyph
        let segs = f.glyph_segments_styled(MathStyle::Variable, 'x', 100.0, 0.0, 0.0, 1);
        assert!(
            segs.is_some() && !segs.unwrap().is_empty(),
            "ssty glyph must produce valid outline segments"
        );
    }

    #[test]
    fn test_glyph_bounds_outlines_polygons_and_raster() {
        let f = fonts();

        // math_glyph_bbox: visible math glyph vs space
        let sum_gid = f
            .glyph_id_for_math(MathStyle::ExtraMath, '∑')
            .expect("summation glyph must exist");
        let (xmin, ymin, xmax, ymax) = f
            .math_glyph_bbox(sum_gid)
            .expect("summation must have non-empty bbox");
        assert!(xmin < xmax && ymin < ymax);
        let space_gid = f
            .glyph_id_for_math(MathStyle::Text, ' ')
            .expect("space glyph must exist");
        assert_eq!(f.math_glyph_bbox(space_gid), None);

        // ink and ink_x: non-zero for 'g' (has both ascender and descender), zero for space
        let (asc, desc) = f.ink(MathStyle::Variable, 'g', 100.0);
        assert!(
            asc > 20.0 && desc > 10.0,
            "expected g ink asc/desc, got ({asc}, {desc})"
        );
        let (lx, rx) = f.ink_x(MathStyle::Variable, 'g', 100.0);
        assert!(rx > lx);
        assert_eq!(f.ink(MathStyle::Text, ' ', 100.0), (0.0, 0.0));
        assert_eq!(f.ink_x(MathStyle::Text, ' ', 100.0), (0.0, 0.0));

        // glyph_segments_styled: orientation (+y down: top of 'A' is negative relative to baseline y=0)
        assert!(f
            .glyph_segments(MathStyle::Text, ' ', 100.0, 0.0, 0.0)
            .is_none());
        let a_segs = f
            .glyph_segments(MathStyle::Text, 'A', 100.0, 10.0, 100.0)
            .expect("A must have outline segments");
        assert!(matches!(a_segs.first(), Some(Seg::Move(_, _))));
        assert!(matches!(a_segs.last(), Some(Seg::Close)));
        assert!(a_segs
            .iter()
            .any(|s| matches!(s, Seg::Line(_, y) | Seg::Move(_, y) if *y < 100.0)));

        // glyph_polys and math_glyph_polys with anisotropic scale
        let polys_1x = f.glyph_polys(MathStyle::ExtraMath, '∑', 100.0, 100.0, 50.0, 100.0);
        let polys_2x = f.glyph_polys(MathStyle::ExtraMath, '∑', 200.0, 100.0, 50.0, 100.0);
        assert!(!polys_1x.is_empty());
        let span = |ps: &[Vec<(f32, f32)>]| -> f32 {
            let mut mn = f32::INFINITY;
            let mut mx = f32::NEG_INFINITY;
            for p in ps.iter().flatten() {
                mn = mn.min(p.0);
                mx = mx.max(p.0);
            }
            mx - mn
        };
        assert!((span(&polys_2x) - 2.0 * span(&polys_1x)).abs() < 0.5);
        assert!(!f
            .math_glyph_polys(sum_gid, 100.0, 100.0, 50.0, 100.0)
            .is_empty());

        // draw_styled produces non-zero coverage for 'x' and zero for space
        let mut canvas = Canvas::new(120, 120);
        f.draw_styled(MathStyle::Variable, 'x', 60.0, 20.0, 80.0, &mut canvas, 0);
        let filled: f32 = canvas.cov.iter().sum();
        assert!(
            filled > 50.0,
            "expected visible raster coverage for 'x', got {filled}"
        );

        let mut blank = Canvas::new(40, 40);
        f.draw_styled(MathStyle::Text, ' ', 40.0, 10.0, 20.0, &mut blank, 0);
        assert_eq!(blank.cov.iter().sum::<f32>(), 0.0);
    }
}
