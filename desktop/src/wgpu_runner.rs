use crate::{app::App, instance::Instance, software_painter::SoftwarePainter, Event, EventSender};
use eframe::{egui, egui_wgpu, App as _};
use egui_winit::ActionRequested;
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::{mpsc::Receiver, Arc},
    time::Instant,
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

pub(crate) enum RunnerBackend {
    Wgpu {
        config: Box<egui_wgpu::WgpuConfiguration>,
        force: bool,
    },
    Software,
}

enum RunnerPainter {
    Wgpu(Box<egui_wgpu::winit::Painter>),
    Software(Box<SoftwarePainter>),
}

impl RunnerPainter {
    fn max_texture_side(&self) -> Option<usize> {
        match self {
            Self::Wgpu(painter) => painter.max_texture_side(),
            Self::Software(painter) => Some(painter.max_texture_side()),
        }
    }

    fn on_window_resized(&mut self, width: NonZeroU32, height: NonZeroU32) {
        match self {
            Self::Wgpu(painter) => painter.on_window_resized(egui::ViewportId::ROOT, width, height),
            Self::Software(painter) => painter.on_window_resized(width, height),
        }
    }

    fn paint_and_update_textures(
        &mut self,
        pixels_per_point: f32,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &mut egui::TexturesDelta,
        window: &Arc<Window>,
    ) {
        match self {
            Self::Wgpu(painter) => {
                painter.paint_and_update_textures(
                    egui::ViewportId::ROOT,
                    pixels_per_point,
                    clear_color,
                    clipped_primitives,
                    textures_delta,
                    Vec::new(),
                    window,
                );
            }
            Self::Software(painter) => {
                painter.paint_and_update_textures(
                    pixels_per_point,
                    clear_color,
                    clipped_primitives,
                    textures_delta,
                    window,
                );
            }
        }
    }

    fn destroy(&mut self) {
        if let Self::Wgpu(painter) = self {
            painter.destroy();
        }
    }
}

type RunnerInit = (Instance, EventSender<Event>, Receiver<Event>, Vec<String>);

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
    painter: RunnerPainter,
    viewport_info: egui::ViewportInfo,
    actions_requested: Vec<ActionRequested>,
    app: App,
    beginning: Instant,
    last_cursor_phys: Option<[f64; 2]>,
    cursor_in_window: bool,
    next_repaint_time: Option<Instant>,
    app_requested_redraw: bool,
    viewport_dirty: bool,
    pending_deltas: egui::TexturesDelta,
    last_shapes: Option<Vec<egui::epaint::ClippedShape>>,
    last_primitives: Option<Vec<egui::epaint::ClippedPrimitive>>,
    last_surface_dims: Option<[u32; 2]>,
    last_ppp: Option<f32>,
    last_clear_color: Option<[f32; 4]>,
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

    fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
        let os_exposure = !std::mem::take(&mut self.app_requested_redraw);
        self.next_repaint_time = None;

        let inner_size = self.window.inner_size();
        let surface_dims = [inner_size.width, inner_size.height];
        let ppp_now = egui_winit::pixels_per_point(&self.egui_ctx, &self.window);
        let clear_color_now = self.app.clear_color(&self.egui_ctx.global_style().visuals);

        let can_reuse_primitives = os_exposure
            && self.shown
            && !self.viewport_dirty
            && !self.force_paint
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
            // On Windows, DWM retains both DX12 swapchains and GDI surfaces across window moves
            // and WM_PAINT. Re-rasterizing unchanged primitives during dragging only burns CPU.
            #[cfg(target_os = "windows")]
            {
                return;
            }
            #[cfg(not(target_os = "windows"))]
            {
                if let Some(primitives) = self.last_primitives.as_ref() {
                    self.painter.paint_and_update_textures(
                        ppp_now,
                        clear_color_now,
                        primitives,
                        &mut self.pending_deltas,
                        &self.window,
                    );
                }
                return;
            }
        }

        #[cfg(not(target_os = "windows"))]
        if os_exposure {
            self.force_paint = true;
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

        let pointer_only_events = !raw_input.events.is_empty()
            && raw_input
                .events
                .iter()
                .all(|e| matches!(e, egui::Event::PointerMoved(_) | egui::Event::PointerGone));

        let close_requested = raw_input.viewport().close_requested();
        let full_output = self.egui_ctx.run_ui(raw_input, |ui| self.app.ui_pass(ui));

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
        if pointer_only_events && ime_state == self.last_ime_state {
            // Avoid Win32 IMM32 syscalls inside egui_winit when only the mouse pointer moved.
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
                if !self.shown {
                    self.shown = true;
                    self.window.set_visible(true);
                }
                let clipped_primitives = self.egui_ctx.tessellate(shapes.clone(), pixels_per_point);
                self.painter.paint_and_update_textures(
                    pixels_per_point,
                    clear_color,
                    &clipped_primitives,
                    &mut self.pending_deltas,
                    &self.window,
                );
                self.last_shapes = Some(shapes);
                self.last_primitives = Some(clipped_primitives);
                self.last_surface_dims = Some(surface_dims);
                self.last_ppp = Some(pixels_per_point);
                self.last_clear_color = Some(clear_color);
                self.force_paint = false;
            }
        }

        if self.should_close {
            self.egui_ctx.on_exit();
            self.painter.destroy();
            event_loop.exit();
        }
    }
}

struct DesktopRunner {
    viewport_builder: egui::ViewportBuilder,
    backend: Option<RunnerBackend>,
    proxy: EventLoopProxy<RunnerUserEvent>,
    init: Option<RunnerInit>,
    running: Option<Box<RunningState>>,
    error: Option<String>,
}

impl DesktopRunner {
    fn init_running(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let Some((instance, tx, rx, args)) = self.init.take() else {
            return Ok(());
        };
        let backend = self.backend.take().unwrap_or(RunnerBackend::Software);

        let egui_ctx = egui::Context::default();
        egui_ctx.set_embed_viewports(false);
        egui_ctx.options_mut(|o| {
            if let Some(n) = NonZeroUsize::new(2) {
                o.max_passes = n;
            }
        });

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

        let (painter, render_state, is_software) = match backend {
            RunnerBackend::Wgpu { config, force } => {
                let mut config = *config;
                if let egui_wgpu::WgpuSetup::CreateNew(ref mut create_new) = config.wgpu_setup {
                    if create_new.display_handle.is_none() {
                        create_new.display_handle =
                            Some(Box::new(event_loop.owned_display_handle()));
                    }
                }
                let mut wgpu_painter = pollster::block_on(egui_wgpu::winit::Painter::new(
                    egui_ctx.clone(),
                    config,
                    false,
                    egui_wgpu::RendererOptions {
                        msaa_samples: 0,
                        depth_stencil_format: None,
                        dithering: false,
                        ..Default::default()
                    },
                ));
                let surface_res = pollster::block_on(
                    wgpu_painter.set_window(egui::ViewportId::ROOT, Some(Arc::clone(&window))),
                );
                let state = wgpu_painter.render_state();
                let resolved_cpu = state.as_ref().is_some_and(|rs| {
                    let info = rs.adapter.get_info();
                    info.device_type == egui_wgpu::wgpu::DeviceType::Cpu
                        || crate::app::is_software_adapter_name(&info.name)
                });
                if surface_res.is_ok() && state.is_some() && (force || !resolved_cpu) {
                    (RunnerPainter::Wgpu(Box::new(wgpu_painter)), state, false)
                } else if !force {
                    wgpu_painter.destroy();
                    let sw_painter = SoftwarePainter::new(Arc::clone(&window))?;
                    (RunnerPainter::Software(Box::new(sw_painter)), None, true)
                } else if let Err(err) = surface_res {
                    return Err(format!("Failed to initialize wgpu surface: {err}"));
                } else {
                    return Err("No compatible graphics adapter found for wgpu".into());
                }
            }
            RunnerBackend::Software => {
                let sw_painter = SoftwarePainter::new(Arc::clone(&window))?;
                (RunnerPainter::Software(Box::new(sw_painter)), None, true)
            }
        };

        let mut egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            event_loop.system_theme(),
            painter.max_texture_side(),
        );
        egui_winit.init_accesskit(event_loop, &window, self.proxy.clone());

        let proxy = self.proxy.clone();
        egui_ctx.set_request_repaint_callback(move |info| {
            let when = Instant::now() + info.delay;
            let _ = proxy.send_event(RunnerUserEvent::RequestRepaint { when });
        });

        let app = App::new_with_state(
            &egui_ctx,
            render_state.as_ref(),
            is_software,
            instance,
            tx,
            rx,
            args,
        );

        let now = Instant::now();
        self.running = Some(Box::new(RunningState {
            egui_ctx,
            window,
            egui_winit,
            painter,
            viewport_info,
            actions_requested: Vec::new(),
            app,
            beginning: now,
            last_cursor_phys: None,
            cursor_in_window: false,
            next_repaint_time: Some(now),
            app_requested_redraw: false,
            viewport_dirty: true,
            pending_deltas: egui::TexturesDelta::default(),
            last_shapes: None,
            last_primitives: None,
            last_surface_dims: None,
            last_ppp: None,
            last_clear_color: None,
            last_ime_state: None,
            force_paint: true,
            shown: false,
            should_close: false,
        }));
        Ok(())
    }
}

impl ApplicationHandler<RunnerUserEvent> for DesktopRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.listen_device_events(DeviceEvents::Never);
        if self.running.is_none() && self.error.is_none() {
            if let Err(err) = self.init_running(event_loop) {
                self.error = Some(err);
                event_loop.exit();
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {}

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
            RunnerUserEvent::RequestRepaint { when } => {
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
            return;
        }

        match &event {
            WindowEvent::CursorEntered { .. } => {
                running.cursor_in_window = true;
            }
            WindowEvent::CursorLeft { .. } => {
                running.cursor_in_window = false;
                running.last_cursor_phys = None;
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

                if let Some([px, py]) = running.last_cursor_phys {
                    let dx = position.x - px;
                    let dy = position.y - py;
                    if dx * dx + dy * dy < 0.25 {
                        return;
                    }
                }
                running.last_cursor_phys = Some([position.x, position.y]);
            }
            _ => {}
        }

        let mut force_repaint = false;
        match &event {
            WindowEvent::Resized(_) => {
                running.viewport_dirty = true;
                let size = running.window.inner_size();
                if running.last_surface_dims != Some([size.width, size.height]) {
                    if let (Some(w), Some(h)) =
                        (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        running.painter.on_window_resized(w, h);
                        running.force_paint = true;
                        force_repaint = true;
                    }
                }
            }
            WindowEvent::Occluded(is_occluded) => {
                running.viewport_dirty = true;
                running.viewport_info.occluded = Some(*is_occluded);
                if !*is_occluded {
                    #[cfg(not(target_os = "windows"))]
                    {
                        running.force_paint = true;
                    }
                    force_repaint = true;
                }
            }
            WindowEvent::Focused(focused) => {
                running.viewport_dirty = true;
                if *focused {
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
        if response.repaint || force_repaint {
            running.schedule_repaint_now();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        let now = Instant::now();

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

        event_loop.set_control_flow(match running.next_repaint_time {
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
    backend: RunnerBackend,
    instance: Instance,
    tx: EventSender<Event>,
    rx: Receiver<Event>,
    args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<RunnerUserEvent>::with_user_event().build()?;
    event_loop.listen_device_events(DeviceEvents::Never);
    let proxy = event_loop.create_proxy();
    let mut runner = DesktopRunner {
        viewport_builder,
        backend: Some(backend),
        proxy,
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
