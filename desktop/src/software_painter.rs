use eframe::egui;
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use std::{num::NonZeroU32, sync::Arc};
use winit::window::Window;

#[derive(Clone)]
pub(crate) struct SoftwareBackgroundImage {
    pub(crate) revision: u64,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<egui::Color32>,
    pub(crate) has_alpha: bool,
}

impl SoftwareBackgroundImage {
    pub(crate) fn from_color_image(revision: u64, image: &egui::ColorImage) -> Self {
        let has_alpha = image.pixels.iter().any(|p| p.a() < 255);
        Self {
            revision,
            width: image.width(),
            height: image.height(),
            pixels: image.pixels.clone(),
            has_alpha,
        }
    }
}

#[derive(Clone)]
pub(crate) struct SoftwareBackgroundLayer {
    pub(crate) image: Arc<SoftwareBackgroundImage>,
    pub(crate) scrim: egui::Color32,
    pub(crate) base_bg: egui::Color32,
}

#[derive(PartialEq, Eq)]
struct CachedBackgroundKey {
    width: usize,
    height: usize,
    revision: u64,
    scrim: [u8; 4],
    base_bg: [u8; 4],
}

struct CachedBackground {
    key: CachedBackgroundKey,
    pixels_0rgb: Vec<u32>,
}

pub(crate) struct SoftwarePainter {
    _context: softbuffer::Context<Arc<Window>>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    renderer: EguiSoftwareRender,
    dims: Option<[NonZeroU32; 2]>,
    background: Option<SoftwareBackgroundLayer>,
    cached_bg: Option<CachedBackground>,
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
            background: None,
            cached_bg: None,
        })
    }

    pub(crate) fn max_texture_side(&self) -> usize {
        8192
    }

    pub(crate) fn set_background(&mut self, background: Option<SoftwareBackgroundLayer>) {
        if background.is_none() {
            self.cached_bg = None;
        }
        self.background = background;
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

        let slice = &mut buffer[..pixel_count];
        if let Some(layer) = self.background.as_ref() {
            if layer.image.width > 0 && layer.image.height > 0 {
                let key = CachedBackgroundKey {
                    width,
                    height,
                    revision: layer.image.revision,
                    scrim: layer.scrim.to_array(),
                    base_bg: layer.base_bg.to_array(),
                };
                let needs_rebuild = self
                    .cached_bg
                    .as_ref()
                    .is_none_or(|cached| cached.key != key);
                if needs_rebuild {
                    let mut pixels_0rgb = self
                        .cached_bg
                        .take()
                        .map(|c| c.pixels_0rgb)
                        .unwrap_or_default();
                    rasterize_cover_background(layer, width, height, &mut pixels_0rgb);
                    self.cached_bg = Some(CachedBackground { key, pixels_0rgb });
                }
                if let Some(cached) = &self.cached_bg {
                    slice.copy_from_slice(&cached.pixels_0rgb);
                }
            } else {
                slice.fill(clear_color_to_0rgb(clear_color));
            }
        } else {
            slice.fill(clear_color_to_0rgb(clear_color));
        }

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

fn cover_crop_axis(dst_len: usize, uv_start: f32, uv_span: f32, src_len: usize) -> Vec<usize> {
    if dst_len == 0 || src_len == 0 {
        return Vec::new();
    }
    let max_idx = src_len - 1;
    let src_f = src_len as f32;
    let inv_dst = 1.0 / dst_len as f32;
    (0..dst_len)
        .map(|i| {
            let u = uv_start + ((i as f32 + 0.5) * inv_dst) * uv_span;
            let idx = (u * src_f).floor() as isize;
            idx.clamp(0, max_idx as isize) as usize
        })
        .collect()
}

pub(crate) fn rasterize_cover_background(
    layer: &SoftwareBackgroundLayer,
    width: usize,
    height: usize,
    out_0rgb: &mut Vec<u32>,
) {
    let pixel_count = width * height;
    out_0rgb.resize(pixel_count, 0);
    if pixel_count == 0 || layer.image.width == 0 || layer.image.height == 0 {
        return;
    }

    let img_w = layer.image.width;
    let img_h = layer.image.height;
    let view_aspect = width as f32 / height as f32;
    let tex_aspect = img_w as f32 / img_h as f32;
    let (u0, u_span, v0, v_span) = if tex_aspect > view_aspect {
        let uv_w = (view_aspect / tex_aspect).clamp(0.0, 1.0);
        ((1.0 - uv_w) * 0.5, uv_w, 0.0, 1.0)
    } else {
        let uv_h = (tex_aspect / view_aspect).clamp(0.0, 1.0);
        (0.0, 1.0, (1.0 - uv_h) * 0.5, uv_h)
    };

    let x_lut = cover_crop_axis(width, u0, u_span, img_w);
    let y_lut = cover_crop_axis(height, v0, v_span, img_h);

    // `Color32::to_array()` stores premultiplied RGBA bytes.
    let [sr, sg, sb, sa] = layer.scrim.to_array();
    let inv_sa = 255 - sa as u32;
    let lut_r: [u32; 256] = std::array::from_fn(|c| {
        let v = (sr as u32 + ((c as u32 * inv_sa + 127) / 255)).min(255);
        v << 16
    });
    let lut_g: [u32; 256] = std::array::from_fn(|c| {
        let v = (sg as u32 + ((c as u32 * inv_sa + 127) / 255)).min(255);
        v << 8
    });
    let lut_b: [u32; 256] =
        std::array::from_fn(|c| (sb as u32 + ((c as u32 * inv_sa + 127) / 255)).min(255));

    let src_pixels = &layer.image.pixels;
    if !layer.image.has_alpha {
        for (y, &sy) in y_lut.iter().enumerate() {
            let src_row = &src_pixels[sy * img_w..(sy + 1) * img_w];
            let dst_row = &mut out_0rgb[y * width..(y + 1) * width];
            for (dst, &sx) in dst_row.iter_mut().zip(x_lut.iter()) {
                let p = src_row[sx];
                *dst = lut_r[p.r() as usize] | lut_g[p.g() as usize] | lut_b[p.b() as usize];
            }
        }
    } else {
        let [br, bg, bb, _] = layer.base_bg.to_array();
        for (y, &sy) in y_lut.iter().enumerate() {
            let src_row = &src_pixels[sy * img_w..(sy + 1) * img_w];
            let dst_row = &mut out_0rgb[y * width..(y + 1) * width];
            for (dst, &sx) in dst_row.iter_mut().zip(x_lut.iter()) {
                let p = src_row[sx];
                let a = p.a();
                let (r, g, b) = if a == 255 {
                    (p.r(), p.g(), p.b())
                } else {
                    let inv_a = 255 - a as u32;
                    (
                        (p.r() as u32 + ((br as u32 * inv_a + 127) / 255)).min(255) as u8,
                        (p.g() as u32 + ((bg as u32 * inv_a + 127) / 255)).min(255) as u8,
                        (p.b() as u32 + ((bb as u32 * inv_a + 127) / 255)).min(255) as u8,
                    )
                };
                *dst = lut_r[r as usize] | lut_g[g as usize] | lut_b[b as usize];
            }
        }
    }
}

fn clear_color_to_0rgb(clear_color: [f32; 4]) -> u32 {
    let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
    (to_u8(clear_color[0]) << 16) | (to_u8(clear_color[1]) << 8) | to_u8(clear_color[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_color_packs_into_0rgb() {
        assert_eq!(clear_color_to_0rgb([1.0, 0.5, 0.0, 1.0]), 0x00FF_8000);
        assert_eq!(clear_color_to_0rgb([0.0, 0.0, 0.0, 1.0]), 0x0000_0000);
        assert_eq!(clear_color_to_0rgb([1.0, 1.0, 1.0, 1.0]), 0x00FF_FFFF);
    }

    #[test]
    fn cover_background_rasterizer_crops_and_blends_scrim() {
        // 4x2 source: left half red, right half blue; rendered into 2x2 viewport crops left/right edges symmetrically.
        let mut pixels = vec![egui::Color32::from_rgb(200, 0, 0); 8];
        for row in 0..2 {
            pixels[row * 4] = egui::Color32::from_rgb(10, 20, 30);
            pixels[row * 4 + 1] = egui::Color32::from_rgb(40, 80, 120);
            pixels[row * 4 + 2] = egui::Color32::from_rgb(40, 80, 120);
            pixels[row * 4 + 3] = egui::Color32::from_rgb(10, 20, 30);
        }
        let img = egui::ColorImage {
            size: [4, 2],
            source_size: egui::vec2(4.0, 2.0),
            pixels,
        };
        let layer = SoftwareBackgroundLayer {
            image: Arc::new(SoftwareBackgroundImage::from_color_image(1, &img)),
            scrim: egui::Color32::TRANSPARENT,
            base_bg: egui::Color32::BLACK,
        };
        let mut out = Vec::new();
        rasterize_cover_background(&layer, 2, 2, &mut out);
        assert_eq!(out, vec![0x0028_5078; 4]);

        // Semi-transparent scrim dims cleanly.
        let dim_layer = SoftwareBackgroundLayer {
            scrim: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 255),
            ..layer
        };
        rasterize_cover_background(&dim_layer, 2, 2, &mut out);
        assert_eq!(out, vec![0x0000_0000; 4]);
    }
}
