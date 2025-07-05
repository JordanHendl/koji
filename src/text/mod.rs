use crate::renderer::{Vertex, StaticMesh};
use crate::utils::ResourceManager;
use glam::{Mat4, Vec3};
use dashi::utils::Handle;

mod static_text;
mod dynamic_text;
mod font_registry;

pub use static_text::{StaticText, StaticTextCreateInfo};
pub use dynamic_text::{DynamicText, DynamicTextCreateInfo};
pub use font_registry::FontRegistry;
use rusttype::{Font, Scale, point};
use dashi::*;

pub trait TextRenderable {
    fn vertex_buffer(&self) -> Handle<Buffer>;
    fn index_buffer(&self) -> Option<Handle<Buffer>>;
    fn index_count(&self) -> usize;
}

impl TextRenderable for StaticMesh {
    fn vertex_buffer(&self) -> Handle<Buffer> {
        self.vertex_buffer.expect("text vertex buffer")
    }

    fn index_buffer(&self) -> Option<Handle<Buffer>> {
        self.index_buffer
    }

    fn index_count(&self) -> usize {
        self.index_count
    }
}

pub struct TextRenderer2D {
    font: Font<'static>,
}

impl TextRenderer2D {
    pub fn new(registry: &FontRegistry, name: &str) -> Self {
        let font = registry
            .get(name)
            .expect("font not found in registry");
        Self { font }
    }

    /// Access the internal font used for rendering.
    pub fn font(&self) -> &Font<'static> {
        &self.font
    }

    /// Rasterize `text` to an RGBA8 texture and upload via ResourceManager.
    pub fn upload_text_texture(
        &self,
        ctx: &mut Context,
        res: &mut ResourceManager,
        key: &str,
        text: &str,
        scale: f32,
    ) -> Result<[u32; 2], GPUError> {
        let scale = Scale::uniform(scale);
        let v_metrics = self.font.v_metrics(scale);
        let glyphs: Vec<_> = self
            .font
            .layout(text, scale, point(0.0, v_metrics.ascent))
            .collect();
        let width = glyphs
            .iter()
            .rev()
            .filter_map(|g| g.pixel_bounding_box().map(|bb| bb.max.x as i32))
            .next()
            .unwrap_or(0);
        let height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        if width <= 0 || height == 0 {
            let rgba = [0u8; 4];
            let img = ctx.make_image(&ImageInfo {
                debug_name: "text",
                dim: [1, 1, 1],
                format: Format::RGBA8,
                mip_levels: 1,
                layers: 1,
                initial_data: Some(&rgba),
            })?;
            let view = ctx.make_image_view(&ImageViewInfo { img, ..Default::default() })?;
            let sampler = ctx.make_sampler(&SamplerInfo::default())?;
            res.register_combined(key, img, view, [1, 1], sampler);
            return Ok([1, 1]);
        }
        let mut image = vec![0u8; width as usize * height as usize];
        for g in glyphs {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, v| {
                    let x = (x as i32 + bb.min.x) as usize;
                    let y = (y as i32 + bb.min.y) as usize;
                    let idx = y * width as usize + x;
                    image[idx] = (v * 255.0) as u8;
                });
            }
        }
        let mut rgba = vec![0u8; image.len() * 4];
        for (i, a) in image.iter().enumerate() {
            rgba[i * 4] = 255;
            rgba[i * 4 + 1] = 255;
            rgba[i * 4 + 2] = 255;
            rgba[i * 4 + 3] = *a;
        }
        let img = ctx.make_image(&ImageInfo {
                debug_name: "text",
                dim: [width as u32, height as u32, 1],
                format: Format::RGBA8,
                mip_levels: 1,
                layers: 1,
                initial_data: Some(&rgba),
            })?;
        let view = ctx.make_image_view(&ImageViewInfo { img, ..Default::default() })?;
        let sampler = ctx.make_sampler(&SamplerInfo::default())?;
        res.register_combined(key, img, view, [width as u32, height as u32], sampler);
        Ok([width as u32, height as u32])
    }

    /// Create a quad mesh covering the text dimensions.
    pub fn make_quad(&self, dim: [u32; 2], pos: [f32; 2]) -> StaticMesh {
        let w = dim[0] as f32;
        let h = dim[1] as f32;
        let verts = vec![
            Vertex { position: [pos[0], pos[1] - h, 0.0], normal: [0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[0.0,1.0], color:[1.0;4]},
            Vertex { position: [pos[0] + w, pos[1] - h, 0.0], normal:[0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[1.0,1.0], color:[1.0;4]},
            Vertex { position: [pos[0] + w, pos[1], 0.0], normal:[0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[1.0,0.0], color:[1.0;4]},
            Vertex { position: [pos[0], pos[1], 0.0], normal:[0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[0.0,0.0], color:[1.0;4]},
        ];
        let indices = vec![0u32,1,2,2,3,0];
        StaticMesh {
            material_id: "text".into(),
            vertices: verts,
            indices: Some(indices),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    /// Create a quad mesh covering the text dimensions in NDC space.
    pub fn make_quad_ndc(&self, dim: [u32; 2], pos: [f32; 2], screen_size: [f32; 2]) -> StaticMesh {
        let w = 2.0 * dim[0] as f32 / screen_size[0];
        let h = 2.0 * dim[1] as f32 / screen_size[1];
        let verts = vec![
            Vertex { position: [pos[0], pos[1] - h, 0.0], normal: [0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[0.0,1.0], color:[1.0;4]},
            Vertex { position: [pos[0] + w, pos[1] - h, 0.0], normal:[0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[1.0,1.0], color:[1.0;4]},
            Vertex { position: [pos[0] + w, pos[1], 0.0], normal:[0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[1.0,0.0], color:[1.0;4]},
            Vertex { position: [pos[0], pos[1], 0.0], normal:[0.0;3], tangent:[1.0,0.0,0.0,1.0], uv:[0.0,0.0], color:[1.0;4]},
        ];
        let indices = vec![0u32,1,2,2,3,0];
        StaticMesh {
            material_id: "text".into(),
            vertices: verts,
            indices: Some(indices),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    /// Generate a quad per glyph in `text` positioned in NDC space.
    pub fn make_glyph_mesh_ndc(
        &self,
        text: &str,
        scale: f32,
        pos: [f32; 2],
        screen_size: [f32; 2],
    ) -> StaticMesh {
        let scale = Scale::uniform(scale);
        let v_metrics = self.font.v_metrics(scale);
        let glyphs: Vec<_> = self
            .font
            .layout(text, scale, point(0.0, v_metrics.ascent))
            .collect();
        let width = glyphs
            .iter()
            .rev()
            .filter_map(|g| g.pixel_bounding_box().map(|bb| bb.max.x as i32))
            .next()
            .unwrap_or(0) as f32;
        let line_height = (v_metrics.ascent - v_metrics.descent).ceil();

        let mut verts = Vec::with_capacity(glyphs.len() * 4);
        let mut indices = Vec::with_capacity(glyphs.len() * 6);
        let mut cursor = pos[0];
        let sx = screen_size[0];
        let sy = screen_size[1];
        for (i, ch) in text.chars().enumerate() {
            let g = &glyphs[i];
            let adv = self.font.glyph(ch).scaled(scale).h_metrics().advance_width;
            if let Some(bb) = g.pixel_bounding_box() {
                let base = verts.len() as u32;
                let u0 = bb.min.x as f32 / width;
                let u1 = bb.max.x as f32 / width;
                let v0 = bb.max.y as f32 / line_height;
                let v1 = bb.min.y as f32 / line_height;
                let x0 = cursor;
                let x1 = cursor + 2.0 * adv / sx;
                let y0 = pos[1] - 2.0 * line_height as f32 / sy;
                let y1 = pos[1];
                verts.push(Vertex { position: [x0, y0, 0.0], normal: [0.0; 3], tangent: [1.0, 0.0, 0.0, 1.0], uv: [u0, v0], color: [1.0; 4] });
                verts.push(Vertex { position: [x1, y0, 0.0], normal: [0.0; 3], tangent: [1.0, 0.0, 0.0, 1.0], uv: [u1, v0], color: [1.0; 4] });
                verts.push(Vertex { position: [x1, y1, 0.0], normal: [0.0; 3], tangent: [1.0, 0.0, 0.0, 1.0], uv: [u1, v1], color: [1.0; 4] });
                verts.push(Vertex { position: [x0, y1, 0.0], normal: [0.0; 3], tangent: [1.0, 0.0, 0.0, 1.0], uv: [u0, v1], color: [1.0; 4] });
                indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            }
            cursor += 2.0 * adv / sx;
        }

        StaticMesh {
            material_id: "text".into(),
            vertices: verts,
            indices: Some(indices),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    /// Create a quad mesh transformed by `mat`.
    pub fn make_quad_3d(&self, dim: [u32; 2], mat: Mat4) -> StaticMesh {
        let w = dim[0] as f32;
        let h = dim[1] as f32;
        let base = [
            Vec3::new(0.0, -h, 0.0),
            Vec3::new(w, -h, 0.0),
            Vec3::new(w, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let verts: Vec<_> = base
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let pos = mat.transform_point3(*p);
                let uv = match i {
                    0 => [0.0, 1.0],
                    1 => [1.0, 1.0],
                    2 => [1.0, 0.0],
                    _ => [0.0, 0.0],
                };
                Vertex {
                    position: pos.into(),
                    normal: [0.0; 3],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv,
                    color: [1.0; 4],
                }
            })
            .collect();
        let indices = vec![0u32, 1, 2, 2, 3, 0];
        StaticMesh {
            material_id: "text".into(),
            vertices: verts,
            indices: Some(indices),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    /// Create a text mesh either in 2D or 3D space.
    pub fn make_text_mesh(&self, dim: [u32; 2], space: TextSpace) -> StaticMesh {
        match space {
            TextSpace::Dim2(p) => self.make_quad(dim, p),
            TextSpace::Dim3(m) => self.make_quad_3d(dim, m),
        }
    }
}

/// Specify whether text is positioned in 2D or 3D space.
pub enum TextSpace {
    Dim2([f32; 2]),
    Dim3(Mat4),
}
