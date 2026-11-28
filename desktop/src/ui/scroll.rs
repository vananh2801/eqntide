//! Shared mouse-wheel, scroll-chaining suppression, and snapped stepping helpers.

use eframe::egui::{self, Vec2};
use std::ops::RangeInclusive;

/// Clears `smooth_scroll_delta.y` so inner controls or scroll boxes do not chain to outer scroll areas.
pub(crate) fn suppress_scroll_delta_y(ctx: &egui::Context) {
    ctx.input_mut(|input| {
        input.smooth_scroll_delta.y = 0.0;
    });
}

/// Applies discrete `steps` to `current`, snapping to `steps_per_unit` within `range`.
pub(crate) fn step_snapped_value(
    current: f32,
    steps: i32,
    steps_per_unit: f64,
    range: RangeInclusive<f32>,
) -> f32 {
    let step = 1.0 / steps_per_unit;
    let next = f64::from(current) + f64::from(steps) * step;
    let snapped = ((next * steps_per_unit).round() / steps_per_unit) as f32;
    snapped.clamp(*range.start(), *range.end())
}

/// Applies a raw vertical mouse-wheel `delta_y` step (sign-based) snapped to `steps_per_unit`.
pub(crate) fn step_snapped_wheel_delta(
    current: f32,
    wheel_y: f32,
    steps_per_unit: f64,
    range: RangeInclusive<f32>,
) -> f32 {
    let step = 1.0 / steps_per_unit;
    let next = f64::from(current) + f64::from(wheel_y.signum()) * step;
    let snapped = ((next * steps_per_unit).round() / steps_per_unit) as f32;
    snapped.clamp(*range.start(), *range.end())
}

/// Consumes `Ctrl/Cmd+Wheel` scroll events and returns signed discrete step counts.
pub(crate) fn ctrl_wheel_steps(ui: &egui::Ui, accum_id: egui::Id) -> i32 {
    const POINT_STEP_THRESHOLD: f32 = 16.0;
    let (has_ctrl_wheel, discrete_steps, point_delta) = ui.ctx().input(|input| {
        let frame_ctrl = input.modifiers.ctrl || input.modifiers.command;
        let mut has_ctrl_wheel = false;
        let mut discrete_steps = 0_i32;
        let mut point_delta = 0.0_f32;
        for event in &input.raw.events {
            if let egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
                ..
            } = event
            {
                let ctrl_held = frame_ctrl || modifiers.ctrl || modifiers.command;
                if ctrl_held && delta.y.abs() > f32::EPSILON {
                    has_ctrl_wheel = true;
                    match unit {
                        egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => {
                            discrete_steps += delta.y.signum() as i32;
                        }
                        egui::MouseWheelUnit::Point => {
                            point_delta += delta.y;
                        }
                    }
                }
            }
        }
        (has_ctrl_wheel, discrete_steps, point_delta)
    });
    if !has_ctrl_wheel {
        return 0;
    }
    ui.ctx().input_mut(|input| {
        input.smooth_scroll_delta = Vec2::ZERO;
    });
    let mut accum = ui
        .ctx()
        .data_mut(|data| data.get_temp::<f32>(accum_id))
        .unwrap_or(0.0);
    if discrete_steps != 0 {
        accum = 0.0;
    }
    if point_delta.abs() > f32::EPSILON {
        if accum * point_delta < 0.0 {
            accum = 0.0;
        }
        accum += point_delta;
    }
    let point_steps = (accum / POINT_STEP_THRESHOLD).trunc() as i32;
    if point_steps != 0 {
        accum -= point_steps as f32 * POINT_STEP_THRESHOLD;
    }
    ui.ctx().data_mut(|data| data.insert_temp(accum_id, accum));
    discrete_steps + point_steps
}
