//! Design tokens, egui style configuration, adaptive background accents, and math canvas palettes.
//!
//! [`ThemeTokens`] defines accessible Ayu Light and Ayu Dark UI color roles, translucent
//! panel/card derivation for custom background images, dual-tone accent derivation from image
//! color seeds, and global [`egui::Style`] setup. [`CanvasPalette`] maps Light mode
//! (`Ayu light`) and Dark mode (`Ayu dark`) canvas surfaces to high-contrast equation colors.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

pub(crate) const SPACE_XS: f32 = 4.0;
pub(crate) const SPACE_SM: f32 = 6.0;
pub(crate) const SPACE_MD: f32 = 8.0;
pub(crate) const SPACE_LG: f32 = 12.0;
#[allow(dead_code)]
pub(crate) const SPACE_XL: f32 = 16.0;

pub(crate) const RADIUS_CANVAS_OVERLAY: u8 = 2;
pub(crate) const RADIUS_CONTROL: u8 = 6;
pub(crate) const RADIUS_SURFACE: u8 = 8;
pub(crate) const CANVAS_OVERLAY_CORNER_RADIUS: CornerRadius =
    CornerRadius::same(RADIUS_CANVAS_OVERLAY);
pub(crate) const CONTROL_CORNER_RADIUS: CornerRadius = CornerRadius::same(RADIUS_CONTROL);
pub(crate) const SURFACE_CORNER_RADIUS: CornerRadius = CornerRadius::same(RADIUS_SURFACE);

pub(crate) const UI_FONT_SIZE: f32 = 13.0;
pub(crate) const BUTTON_HEIGHT: f32 = 30.0;
pub(crate) const SIDEBAR_TAB_HEIGHT: f32 = 34.0;
pub(crate) const COMPACT_BUTTON_HEIGHT: f32 = 24.0;

const AYU_DARK_TEXT_INK: Color32 = Color32::from_rgb(13, 16, 22);
const MAX_BACKGROUND_SCRIM_ALPHA: f32 = 0.65;
const MIN_SURFACE_ALPHA: f32 = 0.38;

/// Semantic UI color tokens for chrome, panels, controls, selection, and errors.
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
    /// Return the base opaque Ayu token palette for light or dark mode.
    pub(crate) fn for_mode(dark_mode: bool) -> Self {
        Self::for_mode_with_accent(dark_mode, None)
    }

    /// Return the base opaque Ayu token palette with an optional adaptive RGB accent seed.
    pub(crate) fn for_mode_with_accent(dark_mode: bool, accent_seed: Option<[u8; 3]>) -> Self {
        let mut tokens = if dark_mode {
            Self {
                dark_mode: true,
                app_bg: AYU_DARK_TEXT_INK,
                panel_bg: Color32::from_rgb(31, 33, 39),
                card_bg: Color32::from_rgb(37, 40, 48),
                extreme_bg: Color32::from_rgb(9, 12, 16),
                control_bg: Color32::from_rgb(37, 40, 48),
                control_hover_bg: Color32::from_rgb(45, 47, 52),
                control_active_bg: Color32::from_rgb(62, 64, 67),
                control_disabled_bg: Color32::from_rgb(27, 29, 34),
                border: Color32::from_rgb(63, 64, 67),
                border_hover: Color32::from_rgb(92, 95, 102),
                text_primary: Color32::from_rgb(220, 218, 210),
                text_muted: Color32::from_rgb(138, 137, 134),
                text_disabled: Color32::from_rgb(105, 106, 106),
                accent: Color32::from_rgb(255, 180, 84),
                primary_bg: Color32::from_rgb(245, 165, 58),
                primary_hover_bg: Color32::from_rgb(255, 183, 86),
                primary_active_bg: Color32::from_rgb(222, 144, 42),
                primary_fg: AYU_DARK_TEXT_INK,
                selected_bg: Color32::from_rgb(54, 41, 24),
                selected_hover_bg: Color32::from_rgb(68, 52, 30),
                selected_border: Color32::from_rgb(255, 180, 84),
                selected_fg: Color32::from_rgb(255, 206, 138),
                error_fg: Color32::from_rgb(239, 113, 119),
            }
        } else {
            Self {
                dark_mode: false,
                app_bg: Color32::from_rgb(228, 230, 232),
                panel_bg: Color32::from_rgb(236, 236, 237),
                card_bg: Color32::from_rgb(252, 252, 252),
                extreme_bg: Color32::WHITE,
                control_bg: Color32::from_rgb(252, 252, 252),
                control_hover_bg: Color32::from_rgb(223, 224, 225),
                control_active_bg: Color32::from_rgb(207, 208, 210),
                control_disabled_bg: Color32::from_rgb(229, 230, 231),
                border: Color32::from_rgb(207, 209, 210),
                border_hover: Color32::from_rgb(154, 158, 162),
                text_primary: Color32::from_rgb(46, 50, 56),
                text_muted: Color32::from_rgb(107, 112, 118),
                text_disabled: Color32::from_rgb(158, 162, 166),
                accent: Color32::from_rgb(186, 94, 6),
                primary_bg: Color32::from_rgb(176, 86, 4),
                primary_hover_bg: Color32::from_rgb(154, 74, 3),
                primary_active_bg: Color32::from_rgb(132, 62, 2),
                primary_fg: Color32::WHITE,
                selected_bg: Color32::from_rgb(251, 238, 219),
                selected_hover_bg: Color32::from_rgb(246, 226, 196),
                selected_border: Color32::from_rgb(186, 94, 6),
                selected_fg: Color32::from_rgb(134, 66, 3),
                error_fg: Color32::from_rgb(204, 58, 58),
            }
        };

        if let Some(seed) = accent_seed {
            let derived = derive_ui_accent_roles(dark_mode, seed, tokens.panel_bg);
            tokens.accent = derived.accent;
            tokens.primary_bg = derived.primary_bg;
            tokens.primary_hover_bg = derived.primary_hover_bg;
            tokens.primary_active_bg = derived.primary_active_bg;
            tokens.primary_fg = derived.primary_fg;
            tokens.selected_bg = derived.selected_bg;
            tokens.selected_hover_bg = derived.selected_hover_bg;
            tokens.selected_border = derived.selected_border;
            tokens.selected_fg = derived.selected_fg;
        }
        tokens
    }

    /// Return base or translucent tokens for the current mode, background state, and accent seed.
    pub(crate) fn for_mode_with_background_and_accent(
        dark_mode: bool,
        has_background: bool,
        overlay_opacity: f32,
        accent_seed: Option<[u8; 3]>,
    ) -> Self {
        let base = Self::for_mode_with_accent(dark_mode, accent_seed);
        if has_background {
            base.with_background_opacity(overlay_opacity)
        } else {
            base
        }
    }

    /// Return base tokens or translucent tokens when a custom background image is active.
    #[cfg(test)]
    pub(crate) fn for_mode_with_background(
        dark_mode: bool,
        has_background: bool,
        overlay_opacity: f32,
    ) -> Self {
        Self::for_mode_with_background_and_accent(dark_mode, has_background, overlay_opacity, None)
    }

    /// Apply translucent panel and card fills so a wallpaper stays visible while keeping text legible.
    ///
    /// `panel_bg` and `card_bg` scale across `0.38..=1.00` (becoming completely opaque at `100%`),
    /// while `app_bg` stays transparent so open backdrop regions keep revealing the wallpaper and scrim.
    pub(crate) fn with_background_opacity(mut self, overlay_opacity: f32) -> Self {
        let surface_alpha = surface_alpha_for_opacity(overlay_opacity, 0.70);
        self.app_bg = Color32::TRANSPARENT;
        self.panel_bg = with_unmultiplied_alpha(self.panel_bg, surface_alpha);
        self.card_bg = with_unmultiplied_alpha(self.card_bg, surface_alpha);
        self
    }

    /// Compute the full-window scrim fill painted between the background image and UI chrome.
    ///
    /// Scrim alpha scales across `0.00..=0.65` so the background image remains visible even when
    /// `Overlay opacity` is set to `100%`.
    pub(crate) fn background_scrim(dark_mode: bool, overlay_opacity: f32) -> Color32 {
        let opacity = if overlay_opacity.is_finite() {
            overlay_opacity.clamp(0.0, 1.0)
        } else {
            0.70
        };
        let base = Self::for_mode(dark_mode).app_bg;
        let alpha = (opacity * MAX_BACKGROUND_SCRIM_ALPHA * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha)
    }

    /// Return a muted, fully opaque accent color for disabled accent controls so trailing fills cannot bleed through.
    pub(crate) fn disabled_accent(self) -> Color32 {
        let base_card = Self::for_mode(self.dark_mode).card_bg;
        blend_rgb(self.accent, base_card, 0.58)
    }

    /// Return a subtly accent-tinted hover border for interactive chrome without altering neutral tree lines.
    pub(crate) fn control_hover_border(self) -> Color32 {
        blend_rgb(
            self.border_hover,
            self.accent,
            if self.dark_mode { 0.28 } else { 0.24 },
        )
    }

    /// Build a card-styled `egui::Frame` using surface corners and inner spacing.
    pub(crate) fn group_frame(self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.card_bg)
            .corner_radius(SURFACE_CORNER_RADIUS)
            .stroke(Stroke::new(1.0, self.border))
            .inner_margin(SPACE_LG)
    }

    /// Choose icon foreground tint given `selected` and `enabled` states.
    pub(crate) fn icon_tint(self, selected: bool, enabled: bool) -> Color32 {
        if !enabled {
            self.text_disabled
        } else if selected {
            self.selected_fg
        } else {
            self.text_primary
        }
    }

    /// Install light and dark styles onto the `egui::Context` using the default Ayu accent.
    pub(crate) fn apply_style(egui_ctx: &egui::Context) {
        Self::apply_style_with_accent(egui_ctx, None);
    }

    /// Update accent-derived `egui::Style` colors without altering layout or GPU profile settings.
    pub(crate) fn apply_accent_to_style(egui_ctx: &egui::Context, accent_seed: Option<[u8; 3]>) {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let tokens = Self::for_mode_with_accent(theme == egui::Theme::Dark, accent_seed);
            egui_ctx.style_mut_of(theme, |style| {
                let visuals = &mut style.visuals;
                visuals.selection.bg_fill = tokens.selected_bg;
                visuals.selection.stroke = Stroke::new(1.0, tokens.selected_border);
                visuals.hyperlink_color = tokens.accent;
                visuals.text_cursor.stroke = Stroke::new(1.5, tokens.accent);
                visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.control_hover_border());
                visuals.widgets.active.bg_stroke = Stroke::new(1.0, tokens.accent);
                visuals.widgets.open.bg_stroke = Stroke::new(1.0, tokens.accent);
            });
        }
    }

    /// Install light and dark styles onto the `egui::Context` with an optional accent seed.
    pub(crate) fn apply_style_with_accent(egui_ctx: &egui::Context, accent_seed: Option<[u8; 3]>) {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let tokens = Self::for_mode_with_accent(theme == egui::Theme::Dark, accent_seed);
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
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.control_hover_border());
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
}

/// Concrete canvas background, ink, caret, selection, and scrollbar colors.
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

impl CanvasPalette {
    /// Resolve Ayu Light (`dark_mode == false`) or Ayu Dark (`dark_mode == true`) canvas colors.
    #[cfg(test)]
    pub(crate) fn for_mode(dark_mode: bool) -> Self {
        Self::for_mode_with_accent(dark_mode, None)
    }

    /// Resolve Ayu Light or Ayu Dark canvas colors with an optional adaptive RGB accent seed.
    pub(crate) fn for_mode_with_accent(dark_mode: bool, accent_seed: Option<[u8; 3]>) -> Self {
        let (bg, border, ink) = if dark_mode {
            (
                AYU_DARK_TEXT_INK,
                Color32::from_rgb(63, 64, 67),
                Color32::from_rgb(214, 212, 204),
            )
        } else {
            (
                Color32::from_rgb(252, 252, 252),
                Color32::from_rgb(207, 209, 210),
                Color32::from_rgb(53, 57, 62),
            )
        };

        let ui_tokens = ThemeTokens::for_mode_with_accent(dark_mode, accent_seed);
        let accent = ui_tokens.accent;
        let selection_alpha = if dark_mode { 72 } else { 56 };
        let selection_bg =
            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), selection_alpha);
        let placeholder_stroke = blend_rgb(accent, bg, if dark_mode { 0.16 } else { 0.22 });
        let caret = if dark_mode {
            ui_tokens.selected_fg
        } else {
            ui_tokens.primary_bg
        };
        let scroll_inactive = blend_rgb(accent, bg, if dark_mode { 0.62 } else { 0.56 });
        let scroll_track = if dark_mode {
            blend_rgb(bg, Color32::BLACK, 0.15)
        } else {
            blend_rgb(bg, Color32::from_rgb(225, 228, 232), 0.45)
        };
        let text_caret = if dark_mode {
            Color32::from_rgb(90, 193, 254)
        } else {
            Color32::from_rgb(29, 120, 184)
        };

        CanvasPalette {
            bg,
            border,
            ink,
            placeholder_bg: Color32::TRANSPARENT,
            placeholder_stroke,
            caret,
            selection_bg,
            text_caret,
            ime_fg: ui_tokens.selected_fg,
            scroll_inactive,
            scroll_hover: accent,
            scroll_active: caret,
            scroll_track,
        }
    }

    /// Resolve canvas colors for the current mode, custom background state, canvas opacity, and accent seed.
    pub(crate) fn for_mode_with_background_and_accent(
        dark_mode: bool,
        has_background: bool,
        canvas_opacity: f32,
        accent_seed: Option<[u8; 3]>,
    ) -> Self {
        let base = Self::for_mode_with_accent(dark_mode, accent_seed);
        if has_background {
            base.with_canvas_opacity(canvas_opacity)
        } else {
            base
        }
    }

    /// Apply translucent canvas surface fill (`0.38..=1.00`) over a custom background image.
    pub(crate) fn with_canvas_opacity(mut self, canvas_opacity: f32) -> Self {
        let surface_alpha = surface_alpha_for_opacity(canvas_opacity, 1.0);
        self.bg = with_unmultiplied_alpha(self.bg, surface_alpha);
        self
    }
}

/// Convert an sRGB color triplet into `(hue_deg, saturation, lightness)`.
pub(crate) fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let red = r as f32 / 255.0;
    let green = g as f32 / 255.0;
    let blue = b as f32 / 255.0;
    let max_c = red.max(green).max(blue);
    let min_c = red.min(green).min(blue);
    let delta = max_c - min_c;
    let lum = (max_c + min_c) * 0.5;
    if delta <= 1e-6 {
        return (0.0, 0.0, lum);
    }
    let sat = delta / (1.0 - (2.0 * lum - 1.0).abs()).max(1e-6);
    let hue_raw = if (max_c - red).abs() <= 1e-6 {
        ((green - blue) / delta) % 6.0
    } else if (max_c - green).abs() <= 1e-6 {
        (blue - red) / delta + 2.0
    } else {
        (red - green) / delta + 4.0
    };
    let mut hue = hue_raw * 60.0;
    if hue < 0.0 {
        hue += 360.0;
    }
    (hue, sat.clamp(0.0, 1.0), lum.clamp(0.0, 1.0))
}

struct DerivedAccentRoles {
    accent: Color32,
    primary_bg: Color32,
    primary_hover_bg: Color32,
    primary_active_bg: Color32,
    primary_fg: Color32,
    selected_bg: Color32,
    selected_hover_bg: Color32,
    selected_border: Color32,
    selected_fg: Color32,
}

fn surface_alpha_for_opacity(raw_opacity: f32, default_opacity: f32) -> u8 {
    let opacity = if raw_opacity.is_finite() {
        raw_opacity.clamp(0.0, 1.0)
    } else {
        default_opacity
    };
    ((MIN_SURFACE_ALPHA + (1.0 - MIN_SURFACE_ALPHA) * opacity) * 255.0).round() as u8
}

fn with_unmultiplied_alpha(color: Color32, alpha: u8) -> Color32 {
    let [r, g, b, _] = color.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

fn derive_ui_accent_roles(dark_mode: bool, seed: [u8; 3], panel_bg: Color32) -> DerivedAccentRoles {
    let (hue, raw_sat, _) = rgb_to_hsl(seed[0], seed[1], seed[2]);
    let sat = raw_sat.clamp(0.45, 0.88);
    if dark_mode {
        let mut accent_lum = 0.66_f32;
        let mut accent = hsl_to_color32(hue, sat, accent_lum);
        while contrast_ratio(accent, panel_bg) < 4.5 && accent_lum < 0.86 {
            accent_lum += 0.03;
            accent = hsl_to_color32(hue, sat, accent_lum);
        }

        let primary_lum = 0.58_f32;
        let primary_bg = hsl_to_color32(hue, sat.min(0.84), primary_lum);
        let primary_hover_bg = hsl_to_color32(hue, sat.min(0.86), (primary_lum + 0.07).min(0.82));
        let primary_active_bg = hsl_to_color32(hue, sat.min(0.84), (primary_lum - 0.08).max(0.28));
        let primary_fg = best_contrast_fg(primary_bg);
        let primary_bg = ensure_fg_contrast(hue, sat.min(0.84), primary_lum, primary_fg, dark_mode);

        let selected_bg = hsl_to_color32(hue, (sat * 0.55).clamp(0.28, 0.50), 0.19);
        let selected_hover_bg = hsl_to_color32(hue, (sat * 0.58).clamp(0.30, 0.54), 0.24);
        let mut fg_lum = 0.78_f32;
        let mut selected_fg = hsl_to_color32(hue, sat.min(0.88), fg_lum);
        while contrast_ratio(selected_fg, selected_bg) < 5.0 && fg_lum < 0.92 {
            fg_lum += 0.03;
            selected_fg = hsl_to_color32(hue, sat.min(0.88), fg_lum);
        }

        DerivedAccentRoles {
            accent,
            primary_bg,
            primary_hover_bg,
            primary_active_bg,
            primary_fg,
            selected_bg,
            selected_hover_bg,
            selected_border: accent,
            selected_fg,
        }
    } else {
        let mut accent_lum = 0.39_f32;
        let mut accent = hsl_to_color32(hue, sat, accent_lum);
        while contrast_ratio(accent, panel_bg) < 3.5 && accent_lum > 0.20 {
            accent_lum -= 0.02;
            accent = hsl_to_color32(hue, sat, accent_lum);
        }

        let mut primary_lum = accent_lum.min(0.38);
        let mut primary_bg = hsl_to_color32(hue, sat, primary_lum);
        while contrast_ratio(Color32::WHITE, primary_bg) < 4.6 && primary_lum > 0.16 {
            primary_lum -= 0.02;
            primary_bg = hsl_to_color32(hue, sat, primary_lum);
        }
        let primary_fg = if contrast_ratio(Color32::WHITE, primary_bg) >= 4.5 {
            Color32::WHITE
        } else {
            best_contrast_fg(primary_bg)
        };
        let primary_hover_bg = hsl_to_color32(hue, sat, (primary_lum - 0.05).max(0.12));
        let primary_active_bg = hsl_to_color32(hue, sat, (primary_lum - 0.10).max(0.08));

        let selected_bg = hsl_to_color32(hue, (sat * 0.78).clamp(0.45, 0.75), 0.91);
        let selected_hover_bg = hsl_to_color32(hue, (sat * 0.80).clamp(0.48, 0.78), 0.86);
        let mut fg_lum = 0.30_f32;
        let mut selected_fg = hsl_to_color32(hue, sat, fg_lum);
        while contrast_ratio(selected_fg, selected_bg) < 5.0 && fg_lum > 0.14 {
            fg_lum -= 0.02;
            selected_fg = hsl_to_color32(hue, sat, fg_lum);
        }

        DerivedAccentRoles {
            accent,
            primary_bg,
            primary_hover_bg,
            primary_active_bg,
            primary_fg,
            selected_bg,
            selected_hover_bg,
            selected_border: accent,
            selected_fg,
        }
    }
}

fn best_contrast_fg(bg: Color32) -> Color32 {
    if contrast_ratio(AYU_DARK_TEXT_INK, bg) >= contrast_ratio(Color32::WHITE, bg) {
        AYU_DARK_TEXT_INK
    } else {
        Color32::WHITE
    }
}

fn ensure_fg_contrast(hue: f32, sat: f32, start_lum: f32, fg: Color32, dark_mode: bool) -> Color32 {
    let mut lum = start_lum;
    let mut bg = hsl_to_color32(hue, sat, lum);
    let step = if fg == Color32::WHITE { -0.02 } else { 0.02 };
    for _ in 0..16 {
        if contrast_ratio(fg, bg) >= 4.6 {
            break;
        }
        lum = (lum + step).clamp(if dark_mode { 0.22 } else { 0.14 }, 0.82);
        bg = hsl_to_color32(hue, sat, lum);
    }
    bg
}

fn blend_rgb(first: Color32, second: Color32, weight_second: f32) -> Color32 {
    let weight = weight_second.clamp(0.0, 1.0);
    let inv = 1.0 - weight;
    Color32::from_rgb(
        (first.r() as f32 * inv + second.r() as f32 * weight).round() as u8,
        (first.g() as f32 * inv + second.g() as f32 * weight).round() as u8,
        (first.b() as f32 * inv + second.b() as f32 * weight).round() as u8,
    )
}

fn hsl_to_color32(hue_deg: f32, sat: f32, lum: f32) -> Color32 {
    let hue = ((hue_deg % 360.0) + 360.0) % 360.0;
    let sat = sat.clamp(0.0, 1.0);
    let lum = lum.clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * lum - 1.0).abs()) * sat;
    let sector = hue / 60.0;
    let secondary = chroma * (1.0 - ((sector % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match sector as u32 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let match_lum = lum - chroma * 0.5;
    Color32::from_rgb(
        ((r1 + match_lum).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((g1 + match_lum).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((b1 + match_lum).clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn relative_luminance(color: Color32) -> f32 {
    fn linear(channel: u8) -> f32 {
        let scaled = channel as f32 / 255.0;
        if scaled <= 0.03928 {
            scaled / 12.92
        } else {
            ((scaled + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

fn contrast_ratio(first: Color32, second: Color32) -> f32 {
    let lum_first = relative_luminance(first);
    let lum_second = relative_luminance(second);
    let (hi, lo) = if lum_first >= lum_second {
        (lum_first, lum_second)
    } else {
        (lum_second, lum_first)
    };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blend_over(fg: Color32, bg: Color32) -> Color32 {
        let [r, g, b, alpha] = fg.to_srgba_unmultiplied();
        let alpha_norm = alpha as f32 / 255.0;
        let inv = 1.0 - alpha_norm;
        Color32::from_rgb(
            (r as f32 * alpha_norm + bg.r() as f32 * inv).round() as u8,
            (g as f32 * alpha_norm + bg.g() as f32 * inv).round() as u8,
            (b as f32 * alpha_norm + bg.b() as f32 * inv).round() as u8,
        )
    }

    #[test]
    fn theme_tokens_provide_accessible_contrast_in_light_and_dark_modes() {
        for dark in [false, true] {
            let tokens = ThemeTokens::for_mode(dark);
            assert_eq!(tokens.dark_mode, dark);
            assert!(contrast_ratio(tokens.text_primary, tokens.panel_bg) >= 7.0);
            assert!(contrast_ratio(tokens.text_primary, tokens.control_bg) >= 7.0);
            assert!(contrast_ratio(tokens.selected_fg, tokens.selected_bg) >= 4.5);
            assert!(contrast_ratio(tokens.primary_fg, tokens.primary_bg) >= 4.5);
            assert_ne!(tokens.app_bg, tokens.panel_bg);
            let disabled = tokens.disabled_accent();
            assert_eq!(disabled.a(), 255);
            assert_ne!(disabled, tokens.accent);
            assert_ne!(disabled, tokens.card_bg);
        }
        // Default Ayu accent is warm amber/orange (r > g > b) in both modes.
        let light = ThemeTokens::for_mode(false);
        let dark = ThemeTokens::for_mode(true);
        assert!(light.accent.r() > light.accent.g() && light.accent.g() >= light.accent.b());
        assert!(dark.accent.r() > dark.accent.g() && dark.accent.g() > dark.accent.b());
    }

    #[test]
    fn adaptive_accent_derivation_preserves_contrast_across_diverse_hues() {
        let seeds = [
            [242, 151, 24], // warm amber
            [56, 189, 248], // sky cyan
            [74, 222, 128], // mint green
            [168, 85, 247], // violet
            [244, 63, 94],  // coral red
            [250, 204, 21], // bright yellow
        ];
        for seed in seeds {
            let light = ThemeTokens::for_mode_with_accent(false, Some(seed));
            let dark = ThemeTokens::for_mode_with_accent(true, Some(seed));

            for tokens in [light, dark] {
                assert!(contrast_ratio(tokens.primary_fg, tokens.primary_bg) >= 4.5);
                assert!(contrast_ratio(tokens.selected_fg, tokens.selected_bg) >= 4.5);
                assert!(contrast_ratio(tokens.accent, tokens.panel_bg) >= 3.0);
            }
            assert!(relative_luminance(dark.accent) > relative_luminance(light.accent));
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

            // At 100% overlay opacity, panels/cards are fully opaque while app_bg and the 65% scrim keep the wallpaper visible.
            let full_opacity = ThemeTokens::for_mode_with_background(dark, true, 1.0);
            let full_scrim = ThemeTokens::background_scrim(dark, 1.0);
            assert_eq!(full_opacity.panel_bg, opaque.panel_bg);
            assert_eq!(full_opacity.card_bg, opaque.card_bg);
            assert_eq!(full_opacity.app_bg, Color32::TRANSPARENT);
            assert_eq!(
                full_scrim.a(),
                (MAX_BACKGROUND_SCRIM_ALPHA * 255.0).round() as u8
            );
            assert_ne!(blend_over(full_scrim, Color32::WHITE), opaque.app_bg);
            assert_ne!(blend_over(full_scrim, Color32::BLACK), opaque.app_bg);

            // At 0% overlay opacity, the scrim is transparent while surfaces keep a readable floor.
            let zero_opacity = ThemeTokens::for_mode_with_background(dark, true, 0.0);
            let zero_scrim = ThemeTokens::background_scrim(dark, 0.0);
            assert_eq!(zero_scrim.a(), 0);
            assert_eq!(zero_opacity.app_bg, Color32::TRANSPARENT);
            assert!(zero_opacity.panel_bg.a() > 0 && zero_opacity.panel_bg.a() < 255);
        }
    }

    #[test]
    fn apply_style_configures_radii_cursor_and_zero_hover_expansion() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style_with_accent(&ctx, Some([56, 189, 248]));

        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let expected =
                ThemeTokens::for_mode_with_accent(theme == egui::Theme::Dark, Some([56, 189, 248]));
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
            assert_eq!(style.visuals.hyperlink_color, expected.accent);
            assert_eq!(style.visuals.selection.bg_fill, expected.selected_bg);
            assert_eq!(
                style.visuals.widgets.hovered.bg_stroke.color,
                expected.control_hover_border()
            );
            assert_ne!(
                style.visuals.widgets.hovered.bg_stroke.color,
                expected.border_hover
            );
            assert_eq!(
                style.visuals.interact_cursor,
                Some(egui::CursorIcon::PointingHand)
            );
            let widgets = &style.visuals.widgets;
            for state in [
                &widgets.noninteractive,
                &widgets.inactive,
                &widgets.hovered,
                &widgets.active,
                &widgets.open,
            ] {
                assert_eq!(state.corner_radius, CONTROL_CORNER_RADIUS);
                assert_eq!(state.expansion, 0.0);
            }
        }
    }

    #[test]
    fn canvas_palette_uses_ayu_light_and_dark_with_high_ink_contrast() {
        let light_pal = CanvasPalette::for_mode(false);
        let dark_pal = CanvasPalette::for_mode(true);
        assert_eq!(light_pal.bg, Color32::from_rgb(252, 252, 252));
        assert_eq!(dark_pal.bg, AYU_DARK_TEXT_INK);
        for pal in [light_pal, dark_pal] {
            assert!(contrast_ratio(pal.ink, pal.bg) >= 10.0);
            assert_eq!(pal.placeholder_bg, Color32::TRANSPARENT);
            assert_ne!(pal.placeholder_bg, pal.bg);
        }

        let cyan_dark = CanvasPalette::for_mode_with_accent(true, Some([56, 189, 248]));
        let cyan_light = CanvasPalette::for_mode_with_accent(false, Some([56, 189, 248]));
        assert_ne!(cyan_dark.caret, dark_pal.caret);
        assert_ne!(cyan_light.caret, light_pal.caret);
        assert!(contrast_ratio(cyan_dark.caret, cyan_dark.bg) >= 3.0);
        assert!(contrast_ratio(cyan_light.caret, cyan_light.bg) >= 3.0);
    }

    #[test]
    fn canvas_palette_opacity_scales_surface_alpha_and_stays_opaque_without_background() {
        for dark in [false, true] {
            let base = CanvasPalette::for_mode(dark);
            let solid = CanvasPalette::for_mode_with_background_and_accent(dark, false, 0.0, None);
            assert_eq!(solid, base);
            assert_eq!(solid.bg.a(), 255);

            let full = CanvasPalette::for_mode_with_background_and_accent(dark, true, 1.0, None);
            assert_eq!(full, base);
            assert_eq!(full.bg.a(), 255);

            let non_finite =
                CanvasPalette::for_mode_with_background_and_accent(dark, true, f32::NAN, None);
            assert_eq!(non_finite, base);

            let half = CanvasPalette::for_mode_with_background_and_accent(dark, true, 0.5, None);
            let expected_half = ((0.38_f32 + 0.62_f32 * 0.5_f32) * 255.0_f32).round() as u8;
            assert_eq!(half.bg.a(), expected_half);
            assert_eq!(half.border, base.border);
            assert_eq!(half.ink, base.ink);

            let zero = CanvasPalette::for_mode_with_background_and_accent(dark, true, 0.0, None);
            let expected_zero = (0.38_f32 * 255.0_f32).round() as u8;
            assert_eq!(zero.bg.a(), expected_zero);
        }
    }
}
