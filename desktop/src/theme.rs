use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};
use serde::{Deserialize, Serialize};

pub(crate) const SPACE_XS: f32 = 4.0;
pub(crate) const SPACE_SM: f32 = 6.0;
pub(crate) const SPACE_MD: f32 = 8.0;
pub(crate) const SPACE_LG: f32 = 12.0;
#[allow(dead_code)]
pub(crate) const SPACE_XL: f32 = 16.0;

pub(crate) const RADIUS_CONTROL: u8 = 6;
pub(crate) const RADIUS_SURFACE: u8 = 8;
pub(crate) const CONTROL_CORNER_RADIUS: CornerRadius = CornerRadius::same(RADIUS_CONTROL);
pub(crate) const SURFACE_CORNER_RADIUS: CornerRadius = CornerRadius::same(RADIUS_SURFACE);

pub(crate) const UI_FONT_SIZE: f32 = 13.0;
pub(crate) const BUTTON_HEIGHT: f32 = 30.0;
pub(crate) const SIDEBAR_TAB_HEIGHT: f32 = 34.0;
pub(crate) const COMPACT_BUTTON_HEIGHT: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThemeTokens {
    pub(crate) dark_mode: bool,
    pub(crate) app_bg: Color32,
    pub(crate) panel_bg: Color32,
    pub(crate) card_bg: Color32,
    pub(crate) extreme_bg: Color32,
    pub(crate) control_bg: Color32,
    pub(crate) control_hover_bg: Color32,
    pub(crate) control_active_bg: Color32,
    pub(crate) control_disabled_bg: Color32,
    pub(crate) border: Color32,
    pub(crate) border_hover: Color32,
    pub(crate) text_primary: Color32,
    pub(crate) text_muted: Color32,
    pub(crate) text_disabled: Color32,
    pub(crate) accent: Color32,
    pub(crate) primary_bg: Color32,
    pub(crate) primary_hover_bg: Color32,
    pub(crate) primary_active_bg: Color32,
    pub(crate) primary_fg: Color32,
    pub(crate) selected_bg: Color32,
    pub(crate) selected_hover_bg: Color32,
    pub(crate) selected_border: Color32,
    pub(crate) selected_fg: Color32,
    pub(crate) error_fg: Color32,
}

impl ThemeTokens {
    pub(crate) fn for_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                dark_mode: true,
                app_bg: Color32::from_rgb(22, 26, 32),
                panel_bg: Color32::from_rgb(29, 34, 43),
                card_bg: Color32::from_rgb(36, 42, 53),
                extreme_bg: Color32::from_rgb(19, 23, 29),
                control_bg: Color32::from_rgb(42, 49, 62),
                control_hover_bg: Color32::from_rgb(51, 60, 75),
                control_active_bg: Color32::from_rgb(35, 41, 52),
                control_disabled_bg: Color32::from_rgb(32, 37, 47),
                border: Color32::from_rgb(53, 62, 77),
                border_hover: Color32::from_rgb(86, 102, 126),
                text_primary: Color32::from_rgb(229, 234, 242),
                text_muted: Color32::from_rgb(154, 166, 184),
                text_disabled: Color32::from_rgb(93, 104, 122),
                accent: Color32::from_rgb(76, 168, 232),
                primary_bg: Color32::from_rgb(35, 118, 178),
                primary_hover_bg: Color32::from_rgb(46, 136, 200),
                primary_active_bg: Color32::from_rgb(28, 98, 150),
                primary_fg: Color32::WHITE,
                selected_bg: Color32::from_rgb(30, 52, 72),
                selected_hover_bg: Color32::from_rgb(36, 63, 88),
                selected_border: Color32::from_rgb(76, 168, 232),
                selected_fg: Color32::from_rgb(124, 196, 245),
                error_fg: Color32::from_rgb(242, 123, 123),
            }
        } else {
            Self {
                dark_mode: false,
                app_bg: Color32::from_rgb(239, 242, 246),
                panel_bg: Color32::from_rgb(248, 250, 252),
                card_bg: Color32::WHITE,
                extreme_bg: Color32::WHITE,
                control_bg: Color32::WHITE,
                control_hover_bg: Color32::from_rgb(233, 238, 245),
                control_active_bg: Color32::from_rgb(220, 229, 240),
                control_disabled_bg: Color32::from_rgb(241, 245, 249),
                border: Color32::from_rgb(212, 220, 230),
                border_hover: Color32::from_rgb(148, 168, 190),
                text_primary: Color32::from_rgb(30, 41, 59),
                text_muted: Color32::from_rgb(82, 96, 117),
                text_disabled: Color32::from_rgb(148, 163, 184),
                accent: Color32::from_rgb(29, 120, 184),
                primary_bg: Color32::from_rgb(29, 120, 184),
                primary_hover_bg: Color32::from_rgb(22, 100, 155),
                primary_active_bg: Color32::from_rgb(18, 83, 128),
                primary_fg: Color32::WHITE,
                selected_bg: Color32::from_rgb(226, 240, 250),
                selected_hover_bg: Color32::from_rgb(211, 232, 247),
                selected_border: Color32::from_rgb(29, 120, 184),
                selected_fg: Color32::from_rgb(20, 91, 142),
                error_fg: Color32::from_rgb(200, 55, 55),
            }
        }
    }

    pub(crate) fn from_ui(ui: &egui::Ui) -> Self {
        Self::for_mode(ui.visuals().dark_mode)
    }

    pub(crate) fn for_mode_with_background(
        dark_mode: bool,
        has_background: bool,
        overlay_opacity: f32,
    ) -> Self {
        let base = Self::for_mode(dark_mode);
        if has_background {
            base.with_background_opacity(overlay_opacity)
        } else {
            base
        }
    }

    pub(crate) fn with_background_opacity(mut self, overlay_opacity: f32) -> Self {
        let opacity = if overlay_opacity.is_finite() {
            overlay_opacity.clamp(0.0, 1.0)
        } else {
            0.70
        };
        let surface_alpha = ((0.38 + 0.54 * opacity) * 255.0).round() as u8;
        self.app_bg = Color32::TRANSPARENT;
        self.panel_bg = Color32::from_rgba_unmultiplied(
            self.panel_bg.r(),
            self.panel_bg.g(),
            self.panel_bg.b(),
            surface_alpha,
        );
        self.card_bg = Color32::from_rgba_unmultiplied(
            self.card_bg.r(),
            self.card_bg.g(),
            self.card_bg.b(),
            surface_alpha,
        );
        self
    }

    pub(crate) fn background_scrim(dark_mode: bool, overlay_opacity: f32) -> Color32 {
        let opacity = if overlay_opacity.is_finite() {
            overlay_opacity.clamp(0.0, 1.0)
        } else {
            0.70
        };
        let base = Self::for_mode(dark_mode).app_bg;
        let alpha = (opacity * 0.72 * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha)
    }

    pub(crate) fn apply_style(egui_ctx: &egui::Context) {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let tokens = Self::for_mode(theme == egui::Theme::Dark);
            egui_ctx.style_mut_of(theme, |style| {
                style.text_styles = [
                    (egui::TextStyle::Small, egui::FontId::proportional(11.0)),
                    (
                        egui::TextStyle::Body,
                        egui::FontId::proportional(UI_FONT_SIZE),
                    ),
                    (
                        egui::TextStyle::Button,
                        egui::FontId::proportional(UI_FONT_SIZE),
                    ),
                    (egui::TextStyle::Heading, egui::FontId::proportional(18.0)),
                    (
                        egui::TextStyle::Monospace,
                        egui::FontId::monospace(UI_FONT_SIZE),
                    ),
                ]
                .into();
                style.drag_value_text_style = egui::TextStyle::Button;
                style.spacing.item_spacing = Vec2::splat(SPACE_MD);
                style.spacing.button_padding = Vec2::new(10.0, 6.0);
                style.interaction.show_tooltips_only_when_still = false;
                style.interaction.tooltip_delay = 0.5;

                let visuals = &mut style.visuals;
                visuals.dark_mode = tokens.dark_mode;
                visuals.panel_fill = tokens.panel_bg;
                visuals.window_fill = tokens.card_bg;
                visuals.faint_bg_color = tokens.app_bg;
                visuals.extreme_bg_color = tokens.extreme_bg;
                visuals.code_bg_color = tokens.app_bg;
                visuals.window_stroke = Stroke::new(1.0, tokens.border);
                visuals.window_corner_radius = SURFACE_CORNER_RADIUS;
                visuals.menu_corner_radius = SURFACE_CORNER_RADIUS;

                let shadow = egui::Shadow {
                    offset: [0, 4],
                    blur: 12,
                    spread: 0,
                    color: Color32::from_black_alpha(if tokens.dark_mode { 90 } else { 24 }),
                };
                visuals.window_shadow = shadow;
                visuals.popup_shadow = shadow;

                visuals.selection.bg_fill = tokens.selected_bg;
                visuals.selection.stroke = Stroke::new(1.0, tokens.selected_border);
                visuals.hyperlink_color = tokens.accent;
                visuals.error_fg_color = tokens.error_fg;
                visuals.text_cursor.stroke = Stroke::new(1.5, tokens.accent);
                visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);

                let widgets = &mut visuals.widgets;
                widgets.noninteractive.bg_fill = tokens.panel_bg;
                widgets.noninteractive.weak_bg_fill = tokens.panel_bg;
                widgets.noninteractive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.noninteractive.fg_stroke = Stroke::new(1.0, tokens.text_primary);

                widgets.inactive.bg_fill = tokens.control_bg;
                widgets.inactive.weak_bg_fill = tokens.control_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.text_primary);

                widgets.hovered.bg_fill = tokens.control_hover_bg;
                widgets.hovered.weak_bg_fill = tokens.control_hover_bg;
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.border_hover);
                widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.text_primary);

                widgets.active.bg_fill = tokens.control_active_bg;
                widgets.active.weak_bg_fill = tokens.control_active_bg;
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.accent);
                widgets.active.fg_stroke = Stroke::new(1.0, tokens.text_primary);

                widgets.open.bg_fill = tokens.control_hover_bg;
                widgets.open.weak_bg_fill = tokens.control_hover_bg;
                widgets.open.bg_stroke = Stroke::new(1.0, tokens.accent);
                widgets.open.fg_stroke = Stroke::new(1.0, tokens.text_primary);

                for state in [
                    &mut widgets.noninteractive,
                    &mut widgets.inactive,
                    &mut widgets.hovered,
                    &mut widgets.active,
                    &mut widgets.open,
                ] {
                    state.corner_radius = CONTROL_CORNER_RADIUS;
                    state.expansion = 0.0;
                }
            });
        }
    }

    pub(crate) fn group_frame(self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.card_bg)
            .corner_radius(SURFACE_CORNER_RADIUS)
            .stroke(Stroke::new(1.0, self.border))
            .inner_margin(SPACE_LG)
    }

    pub(crate) fn icon_tint(self, selected: bool, enabled: bool) -> Color32 {
        if !enabled {
            self.text_disabled
        } else if selected {
            self.selected_fg
        } else {
            self.text_primary
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CanvasPreset {
    #[default]
    SoftPaper,
    PureWhite,
    CoolSlate,
    WarmCream,
    DimSlate,
    SoftCharcoal,
    DeepInk,
    #[serde(other)]
    Unknown,
}

pub(crate) const LIGHT_CANVAS_PRESETS: &[CanvasPreset] = &[
    CanvasPreset::SoftPaper,
    CanvasPreset::PureWhite,
    CanvasPreset::CoolSlate,
    CanvasPreset::WarmCream,
];

pub(crate) const DARK_CANVAS_PRESETS: &[CanvasPreset] = &[
    CanvasPreset::SoftPaper,
    CanvasPreset::DimSlate,
    CanvasPreset::SoftCharcoal,
    CanvasPreset::DeepInk,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CanvasPalette {
    pub(crate) bg: Color32,
    pub(crate) border: Color32,
    pub(crate) ink: Color32,
    pub(crate) placeholder_bg: Color32,
    pub(crate) placeholder_stroke: Color32,
    pub(crate) caret: Color32,
    pub(crate) selection_bg: Color32,
    pub(crate) text_caret: Color32,
    pub(crate) ime_fg: Color32,
    pub(crate) scroll_inactive: Color32,
    pub(crate) scroll_hover: Color32,
    pub(crate) scroll_active: Color32,
    pub(crate) scroll_track: Color32,
}

impl CanvasPreset {
    pub(crate) fn sanitize_for_mode(self, dark_mode: bool) -> Self {
        let allowed = if dark_mode {
            DARK_CANVAS_PRESETS
        } else {
            LIGHT_CANVAS_PRESETS
        };
        if allowed.contains(&self) {
            self
        } else {
            Self::SoftPaper
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::SoftPaper | Self::Unknown => "Soft paper",
            Self::PureWhite => "Pure white",
            Self::CoolSlate => "Cool slate",
            Self::WarmCream => "Warm cream",
            Self::DimSlate => "Dim slate",
            Self::SoftCharcoal => "Soft charcoal",
            Self::DeepInk => "Deep ink",
        }
    }

    pub(crate) fn is_dark_surface(self) -> bool {
        matches!(self, Self::DimSlate | Self::SoftCharcoal | Self::DeepInk)
    }

    pub(crate) fn palette(self, dark_mode: bool) -> CanvasPalette {
        let preset = self.sanitize_for_mode(dark_mode);
        let light_border = Color32::from_rgb(212, 220, 230);
        let dark_border = Color32::from_rgb(53, 62, 77);
        let (bg, border, ink) = match preset {
            Self::SoftPaper | Self::Unknown => (
                Color32::from_rgb(250, 249, 245),
                if dark_mode { dark_border } else { light_border },
                Color32::from_rgb(26, 32, 44),
            ),
            Self::PureWhite => (Color32::WHITE, light_border, Color32::from_rgb(17, 24, 39)),
            Self::CoolSlate => (
                Color32::from_rgb(237, 242, 247),
                Color32::from_rgb(203, 213, 225),
                Color32::from_rgb(30, 41, 59),
            ),
            Self::WarmCream => (
                Color32::from_rgb(246, 241, 228),
                Color32::from_rgb(223, 213, 190),
                Color32::from_rgb(41, 37, 36),
            ),
            Self::DimSlate => (
                Color32::from_rgb(30, 36, 46),
                dark_border,
                Color32::from_rgb(230, 236, 245),
            ),
            Self::SoftCharcoal => (
                Color32::from_rgb(37, 41, 50),
                Color32::from_rgb(60, 67, 82),
                Color32::from_rgb(236, 239, 244),
            ),
            Self::DeepInk => (
                Color32::from_rgb(20, 24, 32),
                Color32::from_rgb(46, 55, 70),
                Color32::from_rgb(220, 230, 245),
            ),
        };

        if preset.is_dark_surface() {
            CanvasPalette {
                bg,
                border,
                ink,
                placeholder_bg: Color32::from_rgb(33, 52, 74),
                placeholder_stroke: Color32::from_rgb(92, 168, 224),
                caret: Color32::from_rgb(100, 182, 238),
                selection_bg: Color32::from_rgba_unmultiplied(76, 168, 232, 72),
                text_caret: Color32::from_rgb(199, 154, 77),
                ime_fg: Color32::from_rgb(124, 196, 245),
                scroll_inactive: Color32::from_rgb(76, 94, 118),
                scroll_hover: Color32::from_rgb(76, 168, 232),
                scroll_active: Color32::from_rgb(100, 182, 238),
                scroll_track: Color32::from_rgb(27, 33, 43),
            }
        } else {
            CanvasPalette {
                bg,
                border,
                ink,
                placeholder_bg: Color32::from_rgb(232, 243, 253),
                placeholder_stroke: Color32::from_rgb(100, 168, 218),
                caret: Color32::from_rgb(22, 100, 155),
                selection_bg: Color32::from_rgba_unmultiplied(29, 120, 184, 56),
                text_caret: Color32::from_rgb(171, 115, 37),
                ime_fg: Color32::from_rgb(22, 100, 155),
                scroll_inactive: Color32::from_rgb(148, 178, 206),
                scroll_hover: Color32::from_rgb(29, 120, 184),
                scroll_active: Color32::from_rgb(22, 100, 155),
                scroll_track: Color32::from_rgb(235, 242, 249),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relative_luminance(c: Color32) -> f32 {
        fn linear(channel: u8) -> f32 {
            let s = channel as f32 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
    }

    fn contrast_ratio(a: Color32, b: Color32) -> f32 {
        let la = relative_luminance(a);
        let lb = relative_luminance(b);
        let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    fn blend_over(fg: Color32, bg: Color32) -> Color32 {
        let [r, g, b, alpha] = fg.to_srgba_unmultiplied();
        let a = alpha as f32 / 255.0;
        let inv = 1.0 - a;
        Color32::from_rgb(
            (r as f32 * a + bg.r() as f32 * inv).round() as u8,
            (g as f32 * a + bg.g() as f32 * inv).round() as u8,
            (b as f32 * a + bg.b() as f32 * inv).round() as u8,
        )
    }

    #[test]
    fn theme_tokens_provide_accessible_contrast_in_light_and_dark_modes() {
        for dark in [false, true] {
            let t = ThemeTokens::for_mode(dark);
            assert_eq!(t.dark_mode, dark);
            assert!(contrast_ratio(t.text_primary, t.panel_bg) >= 7.0);
            assert!(contrast_ratio(t.text_primary, t.control_bg) >= 7.0);
            assert!(contrast_ratio(t.selected_fg, t.selected_bg) >= 4.5);
            assert!(contrast_ratio(t.primary_fg, t.primary_bg) >= 4.5);
            assert_ne!(t.app_bg, t.panel_bg);
        }
    }

    #[test]
    fn translucent_background_tokens_preserve_contrast_and_opaque_fallback() {
        for dark in [false, true] {
            let opaque = ThemeTokens::for_mode_with_background(dark, false, 0.70);
            assert_eq!(opaque, ThemeTokens::for_mode(dark));
            assert_eq!(opaque.panel_bg.a(), 255);
            assert_eq!(opaque.card_bg.a(), 255);

            let translucent = ThemeTokens::for_mode_with_background(dark, true, 0.70);
            let scrim = ThemeTokens::background_scrim(dark, 0.70);
            assert_eq!(translucent.app_bg, Color32::TRANSPARENT);
            assert!(translucent.panel_bg.a() < 255 && translucent.panel_bg.a() > 0);
            assert!(translucent.card_bg.a() < 255 && translucent.card_bg.a() > 0);

            for extreme_photo in [Color32::BLACK, Color32::WHITE] {
                let after_scrim = blend_over(scrim, extreme_photo);
                let after_panel = blend_over(translucent.panel_bg, after_scrim);
                let after_card = blend_over(translucent.card_bg, after_scrim);
                assert!(contrast_ratio(translucent.text_primary, after_panel) >= 7.0);
                assert!(contrast_ratio(translucent.text_primary, after_card) >= 7.0);
            }
        }
    }

    #[test]
    fn apply_style_configures_radii_cursor_and_zero_hover_expansion() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);

        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let style = ctx.style_of(theme);
            assert_eq!(
                style.text_styles.get(&egui::TextStyle::Button),
                Some(&egui::FontId::proportional(UI_FONT_SIZE))
            );
            assert_eq!(
                style.text_styles.get(&egui::TextStyle::Body),
                Some(&egui::FontId::proportional(UI_FONT_SIZE))
            );
            assert_eq!(style.drag_value_text_style, egui::TextStyle::Button);
            assert_eq!(style.visuals.window_corner_radius, SURFACE_CORNER_RADIUS);
            assert_eq!(style.visuals.menu_corner_radius, SURFACE_CORNER_RADIUS);
            assert_eq!(
                style.visuals.interact_cursor,
                Some(egui::CursorIcon::PointingHand)
            );
            let w = &style.visuals.widgets;
            for state in [
                &w.noninteractive,
                &w.inactive,
                &w.hovered,
                &w.active,
                &w.open,
            ] {
                assert_eq!(state.corner_radius, CONTROL_CORNER_RADIUS);
                assert_eq!(state.expansion, 0.0);
            }
        }
    }

    #[test]
    fn canvas_presets_sanitize_per_mode_and_keep_high_ink_contrast() {
        assert_eq!(CanvasPreset::default(), CanvasPreset::SoftPaper);
        assert_eq!(
            CanvasPreset::DimSlate.sanitize_for_mode(false),
            CanvasPreset::SoftPaper
        );
        assert_eq!(
            CanvasPreset::PureWhite.sanitize_for_mode(true),
            CanvasPreset::SoftPaper
        );
        assert_eq!(
            CanvasPreset::Unknown.sanitize_for_mode(true),
            CanvasPreset::SoftPaper
        );

        for &preset in LIGHT_CANVAS_PRESETS {
            let pal = preset.palette(false);
            assert!(!preset.is_dark_surface());
            assert!(contrast_ratio(pal.ink, pal.bg) >= 10.0);
            assert_ne!(pal.placeholder_bg, pal.bg);
        }
        for &preset in DARK_CANVAS_PRESETS {
            let pal = preset.palette(true);
            assert!(contrast_ratio(pal.ink, pal.bg) >= 10.0);
            assert_ne!(pal.placeholder_bg, pal.bg);
            if preset == CanvasPreset::SoftPaper {
                assert!(!preset.is_dark_surface());
            } else {
                assert!(preset.is_dark_surface());
            }
        }
    }
}
