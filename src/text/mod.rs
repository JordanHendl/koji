use crate::renderer::{Vertex, StaticMesh};
use crate::utils::ResourceManager;

mod static_text;
mod dynamic_text;

pub use static_text::{StaticText, StaticTextCreateInfo};
pub use dynamic_text::{DynamicText, DynamicTextCreateInfo};
use rusttype::{Font, Scale, point};
use dashi::*;

pub struct TextRenderer2D<'a> {
    font: Font<'a>,
}

impl<'a> TextRenderer2D<'a> {
    pub fn new(font_data: &'a [u8]) -> Self {
        let font = Font::try_from_bytes(font_data).expect("font");
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
}
