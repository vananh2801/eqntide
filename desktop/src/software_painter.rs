use eframe::egui;
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use std::{num::NonZeroU32, sync::Arc};
use winit::window::Window;

pub(crate) struct SoftwarePainter {
    _context: softbuffer::Context<Arc<Window>>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    renderer: EguiSoftwareRender,
    dims: Option<[NonZeroU32; 2]>,
}

impl SoftwarePainter {
    pub(crate) fn new(window: Arc<Window>) -> Result<Self, String> {
        let context = softbuffer::Context::new(Arc::clone(&window))
            .map_err(|e| format!("Failed to create software rendering context: {e}"))?;
        let surface = softbuffer::Surface::new(&context, Arc::clone(&window))
            .map_err(|e| format!("Failed to create software rendering surface: {e}"))?;
        Ok(Self {
            _context: context,
            surface,
            renderer: EguiSoftwareRender::new(ColorFieldOrder::Bgra),
            dims: None,
        })
    }

    pub(crate) fn max_texture_side(&self) -> usize {
        8192
    }

    pub(crate) fn on_window_resized(&mut self, width: NonZeroU32, height: NonZeroU32) {
        if self.dims == Some([width, height]) {
            return;
        }
        if let Err(err) = self.surface.resize(width, height) {
            eprintln!("Failed to resize software surface: {err}");
            self.dims = None;
        } else {
            self.dims = Some([width, height]);
        }
    }

    pub(crate) fn paint_and_update_textures(
        &mut self,
        pixels_per_point: f32,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &mut egui::TexturesDelta,
        window: &Window,
    ) {
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if self.dims != Some([w, h]) {
            self.on_window_resized(w, h);
            if self.dims != Some([w, h]) {
                return;
            }
        }

        let mut buffer = match self.surface.buffer_mut() {
            Ok(buffer) => buffer,
            Err(err) => {
                eprintln!("Failed to lock software backbuffer: {err}");
                return;
            }
        };

        let width = w.get() as usize;
        let height = h.get() as usize;
        let Some(pixel_count) = width.checked_mul(height) else {
            return;
        };
        if buffer.len() < pixel_count {
            return;
        }

        let clear_u32 = clear_color_to_0rgb(clear_color);
        let slice = &mut buffer[..pixel_count];
        slice.fill(clear_u32);

        let pixels: &mut [[u8; 4]] = bytemuck::cast_slice_mut(slice);
        let mut buf_ref = BufferMutRef::new(pixels, width, height);
        self.renderer.render(
            &mut buf_ref,
            clipped_primitives,
            textures_delta,
            pixels_per_point.max(0.5),
        );
        textures_delta.clear();

        window.pre_present_notify();
        if let Err(err) = buffer.present() {
            eprintln!("Failed to present software backbuffer: {err}");
        }
    }
}

fn clear_color_to_0rgb(clear_color: [f32; 4]) -> u32 {
    let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
    (to_u8(clear_color[0]) << 16) | (to_u8(clear_color[1]) << 8) | to_u8(clear_color[2])
}

#[cfg(test)]
mod tests {
    use super::clear_color_to_0rgb;

    #[test]
    fn clear_color_packs_into_0rgb() {
        assert_eq!(clear_color_to_0rgb([1.0, 0.5, 0.0, 1.0]), 0x00FF_8000);
        assert_eq!(clear_color_to_0rgb([0.0, 0.0, 0.0, 1.0]), 0x0000_0000);
        assert_eq!(clear_color_to_0rgb([1.0, 1.0, 1.0, 1.0]), 0x00FF_FFFF);
    }
}
