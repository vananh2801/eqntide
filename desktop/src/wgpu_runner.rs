use crate::{app::App, instance::Instance, Event, EventSender};
use eframe::{egui, egui_wgpu, App as _};
use egui_winit::ActionRequested;
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

type RunnerInit = (Instance, EventSender<Event>, Receiver<Event>, Vec<String>);

const HOVER_SETTLE_DELAY: Duration = Duration::from_millis(45);

#[derive(Debug)]
enum RunnerUserEvent {
    RequestRepaint { when: Instant },
    AccessKit(egui_winit::accesskit_winit::Event),
}

impl From<egui_winit::accesskit_winit::Event> for RunnerUserEvent {
    fn from(event: egui_winit::accesskit_winit::Event) -> Self {
        Self::AccessKit(event)
    }
}

struct RunningState {
    egui_ctx: egui::Context,
    window: Arc<Window>,
    egui_winit: egui_winit::State,
    painter: egui_wgpu::winit::Painter,
    viewport_info: egui::ViewportInfo,
    actions_requested: Vec<ActionRequested>,
    app: App,
    beginning: Instant,
    frame_cap: Option<Duration>,
    next_throttled_frame: Instant,
    pending_cursor: Option<WindowEvent>,
    pending_cursor_deadline: Option<Instant>,
    passive_cursor: Option<WindowEvent>,
    last_cursor_phys: Option<[f64; 2]>,
    cursor_in_window: bool,
    pressed_buttons: Vec<MouseButton>,
    next_repaint_time: Option<Instant>,
    app_requested_redraw: bool,
    pending_app_event: bool,
    pending_expose_only: bool,
    viewport_dirty: bool,
    suppress_zero_delay_repaint: Arc<AtomicBool>,
    pending_deltas: egui::TexturesDelta,
    last_shapes: Option<Vec<egui::epaint::ClippedShape>>,
    last_primitives: Option<Vec<egui::epaint::ClippedPrimitive>>,
    last_surface_dims: Option<[u32; 2]>,
    last_ppp: Option<f32>,
    last_clear_color: Option<[f32; 4]>,
    last_paint_time: Option<Instant>,
    last_ime_state: Option<(egui::Rect, egui::IMEPurpose, bool, u32)>,
    force_paint: bool,
    shown: bool,
    should_close: bool,
}

impl Drop for RunningState {
    fn drop(&mut self) {
        self.pending_deltas.clear();
    }
}

impl RunningState {
    fn schedule_repaint_at(&mut self, when: Instant) {
        self.next_repaint_time = Some(match self.next_repaint_time {
            Some(existing) => existing.min(when),
            None => when,
        });
    }

    fn schedule_repaint_now(&mut self) {
        self.schedule_repaint_at(Instant::now());
    }

    fn flush_passive_cursor(&mut self) {
        if let Some(cursor_ev) = self.passive_cursor.take() {
            let _ = self.egui_winit.on_window_event(&self.window, &cursor_ev);
        }
    }

    fn flush_pending_cursor(&mut self) -> bool {
        self.flush_passive_cursor();
        self.pending_cursor_deadline = None;
        let Some(cursor_ev) = self.pending_cursor.take() else {
            return false;
        };
        let response = self.egui_winit.on_window_event(&self.window, &cursor_ev);
        response.repaint
    }

    fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let os_exposure = !std::mem::take(&mut self.app_requested_redraw);
        let expose_only_candidate = os_exposure || std::mem::take(&mut self.pending_expose_only);
        self.next_repaint_time = None;

        let inner_size = self.window.inner_size();
        let surface_dims = [inner_size.width, inner_size.height];
        let ppp_now = egui_winit::pixels_per_point(&self.egui_ctx, &self.window);
        let clear_color_now = self.app.clear_color(&self.egui_ctx.global_style().visuals);

        let can_reuse_primitives = expose_only_candidate
            && self.shown
            && !self.viewport_dirty
            && !self.force_paint
            && !self.pending_app_event
            && self.pending_cursor.is_none()
            && self.pending_deltas.is_empty()
            && self.egui_winit.egui_input().events.is_empty()
            && self.viewport_info.events.is_empty()
            && self.last_surface_dims == Some(surface_dims)
            && self.last_ppp == Some(ppp_now)
            && self.last_clear_color == Some(clear_color_now)
            && self.last_primitives.is_some();

        if can_reuse_primitives {
            let is_minimized = self.window.is_minimized().unwrap_or(false)
                || inner_size.width == 0
                || inner_size.height == 0;
            let is_occluded = self.viewport_info.occluded == Some(true);
            if is_minimized || is_occluded {
                return;
            }
            // On Windows, DX12 flip-model swapchains are retained by DWM across window moves
            // and WM_PAINT. Re-running software WARP rasterization on unchanged primitives
            // during window dragging only burns CPU.
            #[cfg(target_os = "windows")]
            {
                return;
            }
            #[cfg(not(target_os = "windows"))]
            {
                if let Some(cap) = self.frame_cap {
                    if let Some(last_paint) = self.last_paint_time {
                        let next_allowed = last_paint + cap;
                        if now < next_allowed {
                            self.pending_expose_only = true;
                            self.schedule_repaint_at(next_allowed);
                            return;
                        }
                    }
                }
                if let Some(primitives) = self.last_primitives.as_ref() {
                    self.painter.paint_and_update_textures(
                        egui::ViewportId::ROOT,
                        ppp_now,
                        clear_color_now,
                        primitives,
                        &mut self.pending_deltas,
                        Vec::new(),
                        &self.window,
                    );
                    let painted_at = Instant::now();
                    self.last_paint_time = Some(painted_at);
                    if let Some(cap) = self.frame_cap {
                        self.next_throttled_frame = painted_at + cap;
                    }
                }
                return;
            }
        }

        #[cfg(not(target_os = "windows"))]
        if os_exposure {
            self.force_paint = true;
        }

        self.flush_pending_cursor();
        self.pending_app_event = false;
        if let Some(cap) = self.frame_cap {
            self.next_throttled_frame = now + cap;
        }

        if std::mem::take(&mut self.viewport_dirty) {
            egui_winit::update_viewport_info(
                &mut self.viewport_info,
                &self.egui_ctx,
                &self.window,
                false,
            );
        }

        let is_minimized = self.window.is_minimized().unwrap_or(false)
            || inner_size.width == 0
            || inner_size.height == 0;
        let is_occluded = self.viewport_info.occluded == Some(true);
        let skip_paint = self.shown && (is_minimized || is_occluded);

        let mut raw_input = self.egui_winit.take_egui_input(&self.window);
        raw_input.time = Some(self.beginning.elapsed().as_secs_f64());
        raw_input.viewports =
            std::iter::once((egui::ViewportId::ROOT, self.viewport_info.clone())).collect();

        let close_requested = raw_input.viewport().close_requested();
        let prev_ui_state = self.app.ui_state_token();
        let had_popup = self.egui_ctx.any_popup_open();

        let suppress_echo = self.frame_cap.is_some() && !self.app.has_active_animation();
        if suppress_echo {
            self.suppress_zero_delay_repaint
                .store(true, Ordering::Release);
        }
        let full_output = self.egui_ctx.run_ui(raw_input, |ui| self.app.ui_pass(ui));
        if suppress_echo {
            self.suppress_zero_delay_repaint
                .store(false, Ordering::Release);
            let needs_followup = self.app.has_active_animation()
                || self.app.ui_state_token() != prev_ui_state
                || self.egui_ctx.any_popup_open() != had_popup;
            if needs_followup {
                self.schedule_repaint_at(Instant::now().max(self.next_throttled_frame));
            }
        }

        self.viewport_info.events.clear();

        if close_requested {
            let canceled = full_output
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .is_some_and(|o| o.commands.contains(&egui::ViewportCommand::CancelClose));
            if !canceled {
                self.should_close = true;
            }
        }

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = full_output;

        self.pending_deltas.append(textures_delta);

        let ime_state = platform_output.ime.as_ref().map(|m| {
            (
                m.cursor_rect,
                m.purpose,
                m.should_interrupt_composition,
                pixels_per_point.to_bits(),
            )
        });
        if ime_state == self.last_ime_state {
            // Avoid redundant Win32 IMM32 syscalls inside egui_winit when IME state is unchanged.
            self.egui_ctx.input_mut(|i| i.events.clear());
        }
        self.last_ime_state = ime_state;

        self.egui_winit.handle_platform_output_with_event_loop(
            &self.window,
            event_loop,
            platform_output,
        );

        if let Some(root_out) = viewport_output.get(&egui::ViewportId::ROOT) {
            if !root_out.commands.is_empty() {
                egui_winit::process_viewport_commands(
                    &self.egui_ctx,
                    &mut self.viewport_info,
                    root_out.commands.clone(),
                    &self.window,
                    &mut self.actions_requested,
                );
            }
            if root_out.commands.contains(&egui::ViewportCommand::Close) {
                self.should_close = true;
            }
        }

        let actions = std::mem::take(&mut self.actions_requested);
        for action in actions {
            match action {
                ActionRequested::Screenshot { .. } | ActionRequested::PaintWhileHidden => {}
                ActionRequested::Cut => {
                    self.egui_winit
                        .egui_input_mut()
                        .events
                        .push(egui::Event::Cut);
                    self.schedule_repaint_now();
                }
                ActionRequested::Copy => {
                    self.egui_winit
                        .egui_input_mut()
                        .events
                        .push(egui::Event::Copy);
                    self.schedule_repaint_now();
                }
                ActionRequested::Paste => {
                    if let Some(contents) = self.egui_winit.clipboard_text() {
                        let contents = contents.replace("\r\n", "\n");
                        if !contents.is_empty() {
                            self.egui_winit
                                .egui_input_mut()
                                .events
                                .push(egui::Event::Paste(contents));
                            self.schedule_repaint_now();
                        }
                    } else if let Some(image) = self.egui_winit.clipboard_image() {
                        self.egui_winit
                            .egui_input_mut()
                            .events
                            .push(egui::Event::PasteImage(Arc::new(image)));
                        self.schedule_repaint_now();
                    }
                }
            }
        }

        if !skip_paint {
            let clear_color = self.app.clear_color(&self.egui_ctx.global_style().visuals);

            let needs_paint = !self.shown
                || self.force_paint
                || !self.pending_deltas.is_empty()
                || self.last_surface_dims != Some(surface_dims)
                || self.last_ppp != Some(pixels_per_point)
                || self.last_clear_color != Some(clear_color)
                || self.last_shapes.as_ref() != Some(&shapes);

            if needs_paint {
                let clipped_primitives = self.egui_ctx.tessellate(shapes.clone(), pixels_per_point);
                self.painter.paint_and_update_textures(
                    egui::ViewportId::ROOT,
                    pixels_per_point,
                    clear_color,
                    &clipped_primitives,
                    &mut self.pending_deltas,
                    Vec::new(),
                    &self.window,
                );
                let painted_at = Instant::now();
                self.last_paint_time = Some(painted_at);
                if let Some(cap) = self.frame_cap {
                    self.next_throttled_frame = painted_at + cap;
                }
                self.last_shapes = Some(shapes);
                self.last_primitives = Some(clipped_primitives);
                self.last_surface_dims = Some(surface_dims);
                self.last_ppp = Some(pixels_per_point);
                self.last_clear_color = Some(clear_color);
                self.force_paint = false;

                if !self.shown {
                    self.shown = true;
                    self.window.set_visible(true);
                }
            }
        }

        if self.should_close {
            self.egui_ctx.on_exit();
            self.painter.destroy();
            event_loop.exit();
        }
    }
}

struct WgpuRunner {
    viewport_builder: egui::ViewportBuilder,
    wgpu_config: egui_wgpu::WgpuConfiguration,
    proxy: EventLoopProxy<RunnerUserEvent>,
    urgent_repaint: Arc<AtomicBool>,
    init: Option<RunnerInit>,
    running: Option<RunningState>,
    error: Option<String>,
}

impl WgpuRunner {
    fn init_running(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let Some((instance, tx, rx, args)) = self.init.take() else {
            return Ok(());
        };

        let egui_ctx = egui::Context::default();
        egui_ctx.set_embed_viewports(false);
        egui_ctx.options_mut(|o| {
            if let Some(n) = NonZeroUsize::new(2) {
                o.max_passes = n;
            }
        });

        let mut wgpu_config = self.wgpu_config.clone();
        if let egui_wgpu::WgpuSetup::CreateNew(ref mut create_new) = wgpu_config.wgpu_setup {
            if create_new.display_handle.is_none() {
                create_new.display_handle = Some(Box::new(event_loop.owned_display_handle()));
            }
        }

        let mut painter = pollster::block_on(egui_wgpu::winit::Painter::new(
            egui_ctx.clone(),
            wgpu_config,
            false,
            egui_wgpu::RendererOptions {
                msaa_samples: 0,
                depth_stencil_format: None,
                dithering: false,
                ..Default::default()
            },
        ));

        let builder = self.viewport_builder.clone().with_visible(false);
        let window = egui_winit::create_window(&egui_ctx, event_loop, &builder)
            .map_err(|e| format!("Failed to create window: {e}"))?;
        let window = Arc::new(window);

        let mut viewport_info = egui::ViewportInfo::default();
        egui_winit::update_viewport_info(&mut viewport_info, &egui_ctx, &window, true);
        let ppp = egui_winit::pixels_per_point(&egui_ctx, &window);
        egui_ctx.input_mut(|i| {
            i.raw
                .viewports
                .insert(egui::ViewportId::ROOT, viewport_info.clone());
            i.pixels_per_point = ppp;
        });

        pollster::block_on(painter.set_window(egui::ViewportId::ROOT, Some(Arc::clone(&window))))
            .map_err(|e| format!("Failed to initialize wgpu surface: {e}"))?;

        let render_state = painter.render_state();
        if render_state.is_none() {
            return Err("No compatible graphics adapter found for wgpu".into());
        }

        let mut egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            event_loop.system_theme(),
            painter.max_texture_side(),
        );
        egui_winit.init_accesskit(event_loop, &window, self.proxy.clone());

        let suppress_zero_delay_repaint = Arc::new(AtomicBool::new(false));
        let suppress_for_cb = Arc::clone(&suppress_zero_delay_repaint);
        let urgent_for_cb = Arc::clone(&self.urgent_repaint);
        let proxy = self.proxy.clone();
        egui_ctx.set_request_repaint_callback(move |info| {
            if info.delay.is_zero()
                && suppress_for_cb.load(Ordering::Acquire)
                && !urgent_for_cb.load(Ordering::Acquire)
            {
                return;
            }
            let when = Instant::now() + info.delay;
            let _ = proxy.send_event(RunnerUserEvent::RequestRepaint { when });
        });

        let app = App::new_with_state(&egui_ctx, render_state.as_ref(), instance, tx, rx, args);
        let frame_cap = app.frame_cap();
        let initial_size = window.inner_size();
        let initial_dims = (initial_size.width > 0 && initial_size.height > 0)
            .then_some([initial_size.width, initial_size.height]);

        let now = Instant::now();
        self.running = Some(RunningState {
            egui_ctx,
            window,
            egui_winit,
            painter,
            viewport_info,
            actions_requested: Vec::new(),
            app,
            beginning: now,
            frame_cap,
            next_throttled_frame: now,
            pending_cursor: None,
            pending_cursor_deadline: None,
            passive_cursor: None,
            last_cursor_phys: None,
            cursor_in_window: false,
            pressed_buttons: Vec::new(),
            next_repaint_time: Some(now),
            app_requested_redraw: false,
            pending_app_event: false,
            pending_expose_only: false,
            viewport_dirty: true,
            suppress_zero_delay_repaint,
            pending_deltas: egui::TexturesDelta::default(),
            last_shapes: None,
            last_primitives: None,
            last_surface_dims: initial_dims,
            last_ppp: None,
            last_clear_color: None,
            last_paint_time: None,
            last_ime_state: None,
            force_paint: true,
            shown: false,
            should_close: false,
        });
        Ok(())
    }
}

impl ApplicationHandler<RunnerUserEvent> for WgpuRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.listen_device_events(DeviceEvents::Never);
        if self.running.is_none() && self.error.is_none() {
            if let Err(err) = self.init_running(event_loop) {
                self.error = Some(err);
                event_loop.exit();
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if let Some(running) = self.running.as_mut() {
                let due = running
                    .pending_cursor_deadline
                    .unwrap_or(running.next_throttled_frame);
                if running.pending_cursor.is_some()
                    && Instant::now() >= due
                    && running.flush_pending_cursor()
                {
                    running.schedule_repaint_now();
                }
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let _ = event;
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RunnerUserEvent) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        match event {
            RunnerUserEvent::RequestRepaint { mut when } => {
                let is_urgent = self.urgent_repaint.swap(false, Ordering::AcqRel);
                if is_urgent {
                    running.pending_app_event = true;
                } else if let Some(cap) = running.frame_cap {
                    let min_when = Instant::now() + cap / 2;
                    when = when.max(running.next_throttled_frame).max(min_when);
                }
                running.schedule_repaint_at(when);
            }
            RunnerUserEvent::AccessKit(ak_event) => match &ak_event.window_event {
                egui_winit::accesskit_winit::WindowEvent::InitialTreeRequested => {
                    running.egui_ctx.enable_accesskit();
                    running.force_paint = true;
                    running.schedule_repaint_now();
                }
                egui_winit::accesskit_winit::WindowEvent::ActionRequested(request) => {
                    running
                        .egui_winit
                        .on_accesskit_action_request(request.clone());
                    running.schedule_repaint_now();
                }
                egui_winit::accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                    running.egui_ctx.disable_accesskit();
                }
            },
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(running) = self.running.as_mut() else {
            return;
        };

        if matches!(event, WindowEvent::RedrawRequested) {
            running.render_frame(event_loop);
            return;
        }

        // Moving the window across the desktop does not change client area pixels on Windows/X11/Wayland.
        #[cfg(not(target_os = "macos"))]
        if matches!(event, WindowEvent::Moved(_)) {
            running.pending_cursor = None;
            running.pending_cursor_deadline = None;
            running.passive_cursor = None;
            if !running.pending_app_event && !running.force_paint {
                running.next_repaint_time = None;
            }
            return;
        }

        match &event {
            WindowEvent::CursorEntered { .. } => {
                running.cursor_in_window = true;
            }
            WindowEvent::CursorLeft { .. } => {
                running.cursor_in_window = false;
                running.last_cursor_phys = None;
                if running.frame_cap.is_some()
                    && running.pressed_buttons.is_empty()
                    && !running.app.has_active_overlay()
                    && !running.egui_ctx.any_popup_open()
                {
                    running.flush_passive_cursor();
                    let settle =
                        (Instant::now() + HOVER_SETTLE_DELAY).max(running.next_throttled_frame);
                    running.pending_cursor = Some(event);
                    running.pending_cursor_deadline = Some(settle);
                    return;
                }
                running.pending_cursor = None;
                running.pending_cursor_deadline = None;
                running.flush_passive_cursor();
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    if !running.pressed_buttons.contains(button) {
                        running.pressed_buttons.push(*button);
                    }
                }
                ElementState::Released => {
                    running.pressed_buttons.retain(|b| b != button);
                }
            },
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Released
                && running.frame_cap.is_some()
                && running.pressed_buttons.is_empty() =>
            {
                let _ = running.egui_winit.on_window_event(&running.window, &event);
                running
                    .egui_winit
                    .egui_input_mut()
                    .events
                    .retain(|e| !matches!(e, egui::Event::Key { pressed: false, .. }));
                return;
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Drop winit's synthetic WM_MOUSEMOVE(0, 0) posted on WM_NCLBUTTONDOWN title-bar clicks.
                let is_synthetic_origin = position.x == 0.0
                    && position.y == 0.0
                    && (!running.cursor_in_window
                        || running
                            .last_cursor_phys
                            .is_none_or(|[px, py]| px * px + py * py > 256.0));
                if is_synthetic_origin {
                    return;
                }
                running.cursor_in_window = true;

                let prev_phys = running.last_cursor_phys;
                if let Some([px, py]) = prev_phys {
                    let dx = position.x - px;
                    let dy = position.y - py;
                    if dx * dx + dy * dy < 0.25 {
                        return;
                    }
                }

                let ppp = f64::from(egui_winit::pixels_per_point(
                    &running.egui_ctx,
                    &running.window,
                ));
                let pos_pt = egui::Pos2::new((position.x / ppp) as f32, (position.y / ppp) as f32);
                running.last_cursor_phys = Some([position.x, position.y]);

                if running.frame_cap.is_some() {
                    if !running.viewport_info.focused.unwrap_or(true)
                        && running.pressed_buttons.is_empty()
                    {
                        running.passive_cursor = Some(event);
                        running.pending_cursor = None;
                        running.pending_cursor_deadline = None;
                        return;
                    }
                    let pointer_idle = running.pressed_buttons.is_empty()
                        && !running.app.has_active_overlay()
                        && !running.egui_ctx.any_popup_open()
                        && !running.egui_ctx.input(|i| i.pointer.any_down());

                    if pointer_idle {
                        if let Some(passive_rect) = running.app.passive_hover_rect() {
                            let prev_inside = prev_phys.is_some_and(|[px, py]| {
                                passive_rect
                                    .contains(egui::Pos2::new((px / ppp) as f32, (py / ppp) as f32))
                            });
                            if prev_inside && passive_rect.contains(pos_pt) {
                                running.passive_cursor = Some(event);
                                running.pending_cursor = None;
                                running.pending_cursor_deadline = None;
                                return;
                            }
                        }

                        // Coalesce in-flight pointer movement so software rasterizers do not
                        // repaint intermediate buttons while the cursor is sweeping across the UI.
                        let now = Instant::now();
                        let settle = (now + HOVER_SETTLE_DELAY).max(running.next_throttled_frame);
                        running.pending_cursor = Some(event);
                        running.pending_cursor_deadline = Some(settle);
                        running.passive_cursor = None;
                        return;
                    }

                    let now = Instant::now();
                    let due = now.max(running.next_throttled_frame);
                    running.pending_cursor = Some(event);
                    running.pending_cursor_deadline = Some(due);
                    running.passive_cursor = None;
                    return;
                }

                running.passive_cursor = None;
            }
            _ => {}
        }

        if !matches!(
            event,
            WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::Occluded(_)
        ) && running.flush_pending_cursor()
        {
            running.schedule_repaint_now();
        }

        let mut force_repaint = false;
        match &event {
            WindowEvent::Resized(_) => {
                running.viewport_dirty = true;
                let size = running.window.inner_size();
                let dims = [size.width, size.height];
                if running.last_surface_dims != Some(dims) {
                    if let (Some(w), Some(h)) =
                        (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        running
                            .painter
                            .on_window_resized(egui::ViewportId::ROOT, w, h);
                        running.force_paint = true;
                        force_repaint = true;
                    }
                }
            }
            WindowEvent::Occluded(is_occluded) => {
                running.viewport_dirty = true;
                running.viewport_info.occluded = Some(*is_occluded);
                if !*is_occluded {
                    running.force_paint = true;
                    force_repaint = true;
                }
            }
            WindowEvent::Focused(focused) => {
                running.viewport_dirty = true;
                if !*focused {
                    running.pressed_buttons.clear();
                } else {
                    force_repaint = true;
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                running.viewport_dirty = true;
                running.force_paint = true;
                force_repaint = true;
            }
            WindowEvent::CloseRequested => {
                running
                    .viewport_info
                    .events
                    .push(egui::ViewportEvent::Close);
                force_repaint = true;
            }
            _ => {}
        }

        let response = running.egui_winit.on_window_event(&running.window, &event);
        if force_repaint {
            running.schedule_repaint_now();
        } else if response.repaint {
            if running.frame_cap.is_some() {
                running.schedule_repaint_at(Instant::now().max(running.next_throttled_frame));
            } else {
                running.schedule_repaint_now();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        let now = Instant::now();
        let cursor_due = running
            .pending_cursor_deadline
            .unwrap_or(running.next_throttled_frame);
        if running.pending_cursor.is_some() && now >= cursor_due && running.flush_pending_cursor() {
            running.schedule_repaint_now();
        }

        if let Some(when) = running.next_repaint_time {
            if when <= now {
                running.app_requested_redraw = true;
                running.next_repaint_time = None;
                // On Windows, invisible or minimized windows never receive `RedrawRequested` from
                // `window.request_redraw()`. Render directly so the initial frame calls `set_visible(true)`
                // and background events can un-minimize the window.
                let invisible_or_minimized = !running.shown
                    || running.window.is_visible() == Some(false)
                    || running.window.is_minimized() == Some(true);
                if invisible_or_minimized {
                    running.render_frame(event_loop);
                    if event_loop.exiting() {
                        return;
                    }
                } else {
                    running.window.request_redraw();
                }
            }
        }

        let wake_at = match (
            running.next_repaint_time,
            running.pending_cursor.is_some().then_some(cursor_due),
        ) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        event_loop.set_control_flow(match wake_at {
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(mut running) = self.running.take() {
            running.egui_ctx.on_exit();
            running.painter.destroy();
        }
    }
}

pub(crate) fn run(
    viewport_builder: egui::ViewportBuilder,
    wgpu_config: egui_wgpu::WgpuConfiguration,
    instance: Instance,
    tx: EventSender<Event>,
    rx: Receiver<Event>,
    args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<RunnerUserEvent>::with_user_event().build()?;
    event_loop.listen_device_events(DeviceEvents::Never);
    let proxy = event_loop.create_proxy();
    let urgent_repaint = tx.urgent_handle();
    let mut runner = WgpuRunner {
        viewport_builder,
        wgpu_config,
        proxy,
        urgent_repaint,
        init: Some((instance, tx, rx, args)),
        running: None,
        error: None,
    };
    event_loop.run_app(&mut runner)?;
    if let Some(err) = runner.error {
        return Err(err.into());
    }
    Ok(())
}
