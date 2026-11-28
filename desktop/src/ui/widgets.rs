//! Reusable, `App`-independent styled `egui` controls, cards, level bars, and progress indicators.

use super::{
    icons::{IconCache, UI_ICON_SIZE},
    scroll,
};
use crate::theme::{self, ThemeTokens};
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use std::{ops::RangeInclusive, sync::Arc};

/// Styles and enables a primary action widget using the supplied [`ThemeTokens`].
pub(crate) fn primary_button_with<R>(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    enabled: bool,
    add_button: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        if enabled {
            widgets.inactive.bg_fill = tokens.primary_bg;
            widgets.inactive.weak_bg_fill = tokens.primary_bg;
            widgets.inactive.bg_stroke = Stroke::NONE;
            widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

            widgets.hovered.bg_fill = tokens.primary_hover_bg;
            widgets.hovered.weak_bg_fill = tokens.primary_hover_bg;
            widgets.hovered.bg_stroke = Stroke::NONE;
            widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

            let focus_border = if tokens.dark_mode {
                tokens.text_primary
            } else {
                tokens.primary_fg
            };
            widgets.active.bg_fill = tokens.primary_active_bg;
            widgets.active.weak_bg_fill = tokens.primary_active_bg;
            widgets.active.bg_stroke = Stroke::new(1.0, focus_border);
            widgets.active.fg_stroke = Stroke::new(1.0, tokens.primary_fg);
        } else {
            widgets.noninteractive.bg_fill = tokens.control_bg;
            widgets.noninteractive.weak_bg_fill = tokens.control_bg;
            widgets.noninteractive.bg_stroke = Stroke::NONE;
            widgets.noninteractive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
            widgets.inactive.bg_fill = tokens.control_bg;
            widgets.inactive.weak_bg_fill = tokens.control_bg;
            widgets.inactive.bg_stroke = Stroke::NONE;
            widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
        }
        ui.add_enabled_ui(enabled, add_button).inner
    })
    .inner
}

/// Renders a standard-height primary action button tinted with the active accent.
pub(crate) fn primary_button(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    enabled: bool,
    label: &str,
) -> egui::Response {
    primary_button_with(ui, tokens, enabled, |ui| {
        ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
    })
}

/// Renders a standard-height primary action button with a theme-contrasted Tabler icon.
pub(crate) fn icon_primary_button(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    tokens: ThemeTokens,
    enabled: bool,
    icon: &'static str,
    label: &str,
) -> egui::Response {
    let tint = if enabled {
        tokens.primary_fg
    } else {
        tokens.text_primary
    };
    let image = icons.image(ui.ctx(), icon, UI_ICON_SIZE, tint);
    primary_button_with(ui, tokens, enabled, |ui| {
        ui.add(
            egui::Button::image_and_text(image, label)
                .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
        )
    })
}

/// Renders a standard-height secondary button.
pub(crate) fn standard_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
}

/// Renders a standard-height secondary button with a theme-tinted Tabler icon.
pub(crate) fn icon_standard_button(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    tokens: ThemeTokens,
    icon: &'static str,
    label: &str,
) -> egui::Response {
    let tint = tokens.icon_tint(false, ui.is_enabled());
    let image = icons.image(ui.ctx(), icon, UI_ICON_SIZE, tint);
    ui.add(
        egui::Button::image_and_text(image, label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
    )
}

/// Renders a compact-height (`24px`) secondary button for dense toolbars and tree rows.
pub(crate) fn compact_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(theme::SPACE_MD, 3.0);
        ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::COMPACT_BUTTON_HEIGHT)))
    })
    .inner
}

/// Renders the shared Back button used at the top of secondary views.
pub(crate) fn back_button(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    tokens: ThemeTokens,
) -> egui::Response {
    let response = icon_standard_button(ui, icons, tokens, "chevron-left", "Back");
    ui.add_space(theme::SPACE_MD);
    response
}

/// Applies tinted-accent selection styling around a selectable option widget.
pub(crate) fn with_selectable_style<R>(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    selected: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope(|ui| {
        if selected {
            let visuals = ui.visuals_mut();
            visuals.selection.bg_fill = tokens.selected_bg;
            visuals.selection.stroke = Stroke::new(1.0, tokens.selected_fg);

            let widgets = &mut visuals.widgets;
            widgets.inactive.bg_fill = tokens.selected_bg;
            widgets.inactive.weak_bg_fill = tokens.selected_bg;
            widgets.inactive.bg_stroke = Stroke::NONE;
            widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

            widgets.hovered.bg_fill = tokens.selected_hover_bg;
            widgets.hovered.weak_bg_fill = tokens.selected_hover_bg;
            widgets.hovered.bg_stroke = Stroke::NONE;
            widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

            widgets.active.bg_fill = tokens.selected_hover_bg;
            widgets.active.weak_bg_fill = tokens.selected_hover_bg;
            widgets.active.bg_stroke = Stroke::NONE;
            widgets.active.fg_stroke = Stroke::new(1.0, tokens.selected_fg);
        }
        add_contents(ui)
    })
    .inner
}

/// Applies a `1px` accent border around idle/hovered controls when `active` is true.
pub(crate) fn with_accent_border<R>(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    active: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope(|ui| {
        if active {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.accent);
            widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.accent);
        }
        add_contents(ui)
    })
    .inner
}

/// Keeps custom-painted disabled controls crisp by resetting painter opacity to `1.0` when `!enabled`.
pub(crate) fn without_double_dimming<R>(
    ui: &mut egui::Ui,
    enabled: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if !enabled {
        ui.set_opacity(1.0);
    }
    add_contents(ui)
}

/// Renders a checkbox whose checked or indeterminate box uses the active theme accent while keeping the label neutral.
pub(crate) fn accent_checkbox(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    checked: &mut bool,
    indeterminate: bool,
    label: &str,
) -> egui::Response {
    let enabled = ui.is_enabled();
    let label_color = if enabled {
        tokens.text_primary
    } else {
        tokens.text_disabled
    };
    let is_marked = *checked || indeterminate;
    ui.scope(|ui| {
        if !enabled {
            without_double_dimming(ui, enabled, |ui| {
                let widgets = &mut ui.visuals_mut().widgets;
                let border_color = if is_marked {
                    tokens.disabled_accent()
                } else {
                    tokens.border
                };
                for widget_visuals in [
                    &mut widgets.noninteractive,
                    &mut widgets.inactive,
                    &mut widgets.hovered,
                    &mut widgets.active,
                ] {
                    widget_visuals.bg_fill = tokens.control_disabled_bg;
                    widget_visuals.weak_bg_fill = tokens.control_disabled_bg;
                    widget_visuals.bg_stroke = Stroke::new(1.0, border_color);
                    widget_visuals.fg_stroke = Stroke::new(1.5, border_color);
                }
            });
        } else if is_marked {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.bg_fill = tokens.selected_bg;
            widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.selected_border);
            widgets.inactive.fg_stroke = Stroke::new(1.5, tokens.selected_fg);

            widgets.hovered.bg_fill = tokens.selected_hover_bg;
            widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.selected_border);
            widgets.hovered.fg_stroke = Stroke::new(1.5, tokens.selected_fg);

            widgets.active.bg_fill = tokens.selected_hover_bg;
            widgets.active.bg_stroke = Stroke::new(1.0, tokens.selected_border);
            widgets.active.fg_stroke = Stroke::new(1.5, tokens.selected_fg);
        } else {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.border);
            widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.border_hover);
            widgets.active.bg_stroke = Stroke::new(1.0, tokens.accent);
        }
        let checkbox = if label.is_empty() {
            egui::Checkbox::without_text(checked)
        } else {
            egui::Checkbox::new(checked, egui::RichText::new(label).color(label_color))
        };
        ui.add(checkbox.indeterminate(indeterminate))
    })
    .inner
}

/// Renders a full-width borderless card frame using [`ThemeTokens::group_frame`].
pub(crate) fn full_width_card<R>(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    tokens.group_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        add_contents(ui)
    })
}

/// Renders a Krita-style left-filled `DragValue` with optional hover mouse-wheel stepping.
#[allow(clippy::too_many_arguments)]
pub(crate) fn level_drag_value(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    id_salt: &str,
    speed: f64,
    wheel_steps_per_unit: Option<f64>,
    range: RangeInclusive<f32>,
    value: &mut f32,
    hover_text: &str,
    add_drag: impl FnOnce(&mut egui::Ui, &mut f32, f64) -> egui::Response,
) -> (egui::Response, bool) {
    let enabled = ui.is_enabled();
    let mut wheel_changed = false;
    let response = ui
        .push_id(id_salt, |ui| {
            without_double_dimming(ui, enabled, |_| {});
            ui.style_mut().explanation_tooltips = false;
            let meta_id = ui.scope_id().with("level-meta");
            if enabled {
                if let Some(steps_per_unit) = wheel_steps_per_unit {
                    if let Some((widget_id, widget_rect)) = ui
                        .ctx()
                        .data(|data| data.get_temp::<(egui::Id, Rect)>(meta_id))
                    {
                        let editing_text = ui.memory(|mem| mem.has_focus(widget_id));
                        if !editing_text && ui.rect_contains_pointer(widget_rect) {
                            let wheel_y: f32 = ui.ctx().input(|input| {
                                input
                                    .raw
                                    .events
                                    .iter()
                                    .filter_map(|event| match event {
                                        egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                                        _ => None,
                                    })
                                    .sum()
                            });
                            if wheel_y.abs() > f32::EPSILON {
                                *value = scroll::step_snapped_wheel_delta(
                                    *value,
                                    wheel_y,
                                    steps_per_unit,
                                    range.clone(),
                                );
                                wheel_changed = true;
                                scroll::suppress_scroll_delta_y(ui.ctx());
                            }
                        }
                    }
                }
            }

            let track_idx = ui.painter().add(egui::Shape::Noop);
            let fill_idx = ui.painter().add(egui::Shape::Noop);
            {
                let visuals = ui.visuals_mut();
                let widgets = &mut visuals.widgets;
                widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;
                widgets.noninteractive.weak_bg_fill = egui::Color32::TRANSPARENT;
                widgets.noninteractive.fg_stroke.color = tokens.text_disabled;
                widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
                widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
                visuals.extreme_bg_color = tokens.control_bg;
            }

            let drag_resp = add_drag(ui, value, speed).on_hover_text(hover_text);
            let rect = drag_resp.rect;
            let hovered = enabled && drag_resp.hovered();
            let fraction = if *range.end() > *range.start() {
                ((*value - *range.start()) / (*range.end() - *range.start())).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let track_fill = if !enabled {
                tokens.control_disabled_bg
            } else if hovered {
                tokens.control_hover_bg
            } else {
                tokens.control_bg
            };
            ui.painter().set(
                track_idx,
                egui::Shape::rect_filled(rect, theme::CONTROL_CORNER_RADIUS, track_fill),
            );
            let fill_width = rect.width() * fraction;
            if fill_width > 0.5 {
                let radius = theme::RADIUS_CONTROL;
                let radius_f = f32::from(radius);
                let vertical_inset = if fill_width < radius_f {
                    let dx = radius_f - fill_width;
                    radius_f - (radius_f * radius_f - dx * dx).max(0.0).sqrt()
                } else {
                    0.0
                };
                let fill_rect = Rect::from_min_max(
                    Pos2::new(rect.left(), rect.top() + vertical_inset),
                    Pos2::new(rect.left() + fill_width, rect.bottom() - vertical_inset),
                );
                let corner_radius = if fraction >= 0.999 {
                    theme::CONTROL_CORNER_RADIUS
                } else {
                    egui::CornerRadius {
                        nw: radius,
                        sw: radius,
                        ne: 0,
                        se: 0,
                    }
                };
                ui.painter().set(
                    fill_idx,
                    egui::Shape::rect_filled(
                        fill_rect,
                        corner_radius,
                        tokens.soft_accent_fill(hovered, enabled),
                    ),
                );
            }
            ui.ctx()
                .data_mut(|data| data.insert_temp(meta_id, (drag_resp.id, rect)));
            drag_resp
        })
        .inner;
    (response, wheel_changed)
}

/// Renders a section separator line using the active theme border token without adding borders to disabled widgets.
pub(crate) fn section_separator(ui: &mut egui::Ui, tokens: ThemeTokens) {
    let prev_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.visuals_mut().widgets.noninteractive.bg_stroke = Stroke::new(1.0, tokens.border);
    ui.separator();
    ui.visuals_mut().widgets.noninteractive.bg_stroke = prev_stroke;
}

/// Renders a progress bar with `primary_bg` fill and split-contrast status text.
pub(crate) fn progress_bar(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    width: f32,
    height: f32,
    fraction: f32,
    label: &str,
) {
    let clamped = if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    response.widget_info(|| {
        let mut info =
            egui::WidgetInfo::labeled(egui::Role::ProgressIndicator, ui.is_enabled(), label);
        info.value = Some((f64::from(clamped) * 100.0).floor());
        info
    });
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, theme::CONTROL_CORNER_RADIUS, tokens.extreme_bg);
    let mut fill_rect = None;
    if clamped > 0.0 {
        let min_fill = (2.0 * f32::from(theme::RADIUS_CONTROL)).min(rect.width());
        let fill_width = (rect.width() * clamped).max(min_fill).min(rect.width());
        let bar_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
        painter.rect_filled(bar_rect, theme::CONTROL_CORNER_RADIUS, tokens.primary_bg);
        fill_rect = Some(bar_rect);
    }
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui.fonts_mut(|fonts| {
        fonts.layout_no_wrap(label.to_owned(), font_id, egui::Color32::PLACEHOLDER)
    });
    let text_pos = Pos2::new(
        rect.left() + theme::SPACE_MD,
        rect.center().y - galley.size().y * 0.5,
    );
    if let Some(bar_rect) = fill_rect {
        painter.with_clip_rect(bar_rect.intersect(rect)).galley(
            text_pos,
            Arc::clone(&galley),
            tokens.primary_fg,
        );
        if bar_rect.right() < rect.right() {
            let remaining_rect =
                Rect::from_min_max(Pos2::new(bar_rect.right(), rect.top()), rect.max);
            painter
                .with_clip_rect(remaining_rect.intersect(rect))
                .galley(text_pos, galley, tokens.text_primary);
        }
    } else {
        painter
            .with_clip_rect(rect)
            .galley(text_pos, galley, tokens.text_primary);
    }
}
