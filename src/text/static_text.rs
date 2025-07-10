use crate::renderer::{StaticMesh, Vertex};
use crate::text::{TextRenderer2D, TextRenderable};
use crate::utils::ResourceManager;
use dashi::*;
use dashi::utils::Handle;
use rusttype::{Scale, point};
use std::collections::HashMap;

/// Parameters for constructing [`StaticText`].
pub struct StaticTextCreateInfo<'a> {
    /// The string contents to render
    pub text: &'a str,
    /// Font scale in pixels
    pub scale: f32,
    /// Position of the lower-left corner
    pub pos: [f32; 2],
    /// Resource key for the uploaded texture
    pub key: &'a str,
}

struct GlyphInfo {
    advance: f32,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

struct TextAtlas {
    glyphs: HashMap<char, GlyphInfo>,
    line_height: f32,
    texture_key: String,
    index: u32,
}

impl TextAtlas {
    fn new(
        ctx: &mut Context,
        res: &mut ResourceManager,
        renderer: &mut TextRenderer2D,
        key: &str,
        scale: f32,
    ) -> Result<Self, GPUError> {
        let font = renderer.font();
        let scale = Scale::uniform(scale);
        let v_metrics = font.v_metrics(scale);
        let line_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let chars: Vec<char> = (32u8..=126u8).map(|c| c as char).collect();
        let cols = 16u32;
        let rows = ((chars.len() as u32 + cols - 1) / cols) as u32;
        let mut max_adv = 0f32;
        for &c in &chars {
            let adv = font.glyph(c).scaled(scale).h_metrics().advance_width;
            if adv > max_adv {
                max_adv = adv;
            }
        }
        let cell_w = max_adv.ceil() as u32;
        let cell_h = line_height;
        let atlas_w = cell_w * cols;
        let atlas_h = cell_h * rows;
        let mut image = vec![0u8; (atlas_w * atlas_h) as usize];
        let mut glyphs = HashMap::new();
        for (i, ch) in chars.iter().enumerate() {
            let col = i as u32 % cols;
            let row = i as u32 / cols;
            let x = col * cell_w;
            let y = row * cell_h;
            let g = font
                .glyph(*ch)
                .scaled(scale)
                .positioned(point(x as f32, v_metrics.ascent + y as f32));
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|px, py, v| {
                    let px = (px as i32 + bb.min.x) as usize;
                    let py = (py as i32 + bb.min.y) as usize;
                    let idx = py * atlas_w as usize + px;
                    image[idx] = (v * 255.0) as u8;
                });
            }
            let uv_min = [x as f32 / atlas_w as f32, (y + cell_h) as f32 / atlas_h as f32];
            let uv_max = [(x + cell_w) as f32 / atlas_w as f32, y as f32 / atlas_h as f32];
            let adv = font.glyph(*ch).scaled(scale).h_metrics().advance_width;
            glyphs.insert(*ch, GlyphInfo { advance: adv, uv_min, uv_max });
        }
        let mut rgba = vec![0u8; image.len() * 4];
        for (i, a) in image.iter().enumerate() {
            rgba[i * 4] = 255;
            rgba[i * 4 + 1] = 255;
            rgba[i * 4 + 2] = 255;
            rgba[i * 4 + 3] = *a;
        }
        let img = ctx.make_image(&ImageInfo {
            debug_name: "text_atlas",
            dim: [atlas_w, atlas_h, 1],
            format: Format::RGBA8,
            mip_levels: 1,
            layers: 1,
            initial_data: Some(&rgba),
        })?;
        let view = ctx.make_image_view(&ImageViewInfo { img, ..Default::default() })?;
        let sampler = ctx.make_sampler(&SamplerInfo::default())?;
        res.register_combined(key, img, view, [atlas_w, atlas_h], sampler);
        let index = renderer.add_texture(img, view, sampler, [atlas_w, atlas_h]);
        Ok(Self {
            glyphs,
            line_height: cell_h as f32,
            texture_key: key.into(),
            index,
        })
    }
}

/// Immutable text mesh with pre-generated geometry and glyph texture.
pub struct StaticText {
    /// Quad mesh uploaded to the GPU
    mesh: StaticMesh,
    atlas: TextAtlas,
    /// Resource manager key for the glyph texture
    pub texture_key: String,
    /// Index into the bindless texture array
    pub tex_index: u32,
    /// Dimensions of the generated texture
    dim: [u32; 2],
}

impl TextRenderable for StaticText {
    fn vertex_buffer(&self) -> Handle<Buffer> {
        self.mesh.vertex_buffer.expect("text vertex buffer")
    }

    fn index_buffer(&self) -> Option<Handle<Buffer>> {
        self.mesh.index_buffer
    }

    fn index_count(&self) -> usize {
        self.mesh.index_count
    }
}

impl StaticText {
    /// Create a new `StaticText` object. This uploads a texture for `text`
    /// and creates a quad mesh covering its bounds.
    pub fn new(
        ctx: &mut Context,
        res: &mut ResourceManager,
        renderer: &mut TextRenderer2D,
        info: StaticTextCreateInfo<'_>,
    ) -> Result<Self, GPUError> {
        let atlas = TextAtlas::new(ctx, res, renderer, info.key, info.scale)?;
        let tex_index = atlas.index;
        let mut verts = Vec::with_capacity(info.text.len() * 4);
        let mut inds = Vec::with_capacity(info.text.len() * 6);
        let mut cursor = info.pos[0];
        for ch in info.text.chars() {
            if let Some(g) = atlas.glyphs.get(&ch) {
                let base = verts.len() as u32;
                let adv = g.advance;
                let x0 = cursor;
                let x1 = cursor + adv;
                let y0 = info.pos[1] - atlas.line_height;
                let y1 = info.pos[1];
                let c = [tex_index as f32, 0.0, 0.0, 1.0];
                verts.push(Vertex { position: [x0, y0, 0.0], normal: [0.0;3], tangent: [1.0,0.0,0.0,1.0], uv: [g.uv_min[0], g.uv_max[1]], color: c });
                verts.push(Vertex { position: [x1, y0, 0.0], normal: [0.0;3], tangent: [1.0,0.0,0.0,1.0], uv: [g.uv_max[0], g.uv_max[1]], color: c });
                verts.push(Vertex { position: [x1, y1, 0.0], normal: [0.0;3], tangent: [1.0,0.0,0.0,1.0], uv: [g.uv_max[0], g.uv_min[1]], color: c });
                verts.push(Vertex { position: [x0, y1, 0.0], normal: [0.0;3], tangent: [1.0,0.0,0.0,1.0], uv: [g.uv_min[0], g.uv_min[1]], color: c });
                inds.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
                cursor += adv;
            }
        }
        let dim = [(cursor - info.pos[0]) as u32, atlas.line_height as u32];
        let mut mesh = StaticMesh {
            material_id: "text".into(),
            vertices: verts,
            indices: Some(inds),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        };
        mesh.upload(ctx)?;
        Ok(Self {
            mesh,
            atlas,
            texture_key: info.key.into(),
            tex_index,
            dim,
        })
    }

    /// GPU handle for the vertex buffer.
    pub fn vertex_buffer(&self) -> Handle<Buffer> {
        self.mesh.vertex_buffer.expect("text vertex buffer")
    }

    /// GPU handle for the index buffer.
    pub fn index_buffer(&self) -> Option<Handle<Buffer>> {
        self.mesh.index_buffer
    }

    /// Number of indices for drawing this text.
    pub fn index_count(&self) -> usize {
        self.mesh.index_count
    }

    /// Dimensions of the uploaded glyph texture.
    pub fn dim(&self) -> [u32; 2] {
        self.dim
    }
}
