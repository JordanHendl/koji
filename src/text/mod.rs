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

    /// Rasterize `text` to an RGBA8 texture and upload via ResourceManager.
    pub fn upload_text_texture(
        &self,
        ctx: &mut Context,
        res: &mut ResourceManager,
        key: &str,
        text: &str,
        scale: f32,
    ) -> [u32; 2] {
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
        let img = ctx
            .make_image(&ImageInfo {
                debug_name: "text",
                dim: [width as u32, height as u32, 1],
                format: Format::RGBA8,
                mip_levels: 1,
                layers: 1,
                initial_data: Some(&rgba),
            })
            .unwrap();
        let view = ctx
            .make_image_view(&ImageViewInfo { img, ..Default::default() })
            .unwrap();
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
        res.register_combined(key, img, view, [width as u32, height as u32], sampler);
        [width as u32, height as u32]
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
