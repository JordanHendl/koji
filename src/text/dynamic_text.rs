use crate::renderer::Vertex;
use crate::text::{TextRenderer2D, TextRenderable};
use crate::utils::ResourceManager;
use dashi::utils::Handle;
use dashi::*;
use rusttype::{Scale, point};
use std::collections::HashMap;

/// Parameters for constructing [`DynamicText`].
pub struct DynamicTextCreateInfo<'a> {
    /// Maximum number of UTF-8 characters to allocate for
    pub max_chars: usize,
    /// Initial text contents
    pub text: &'a str,
    /// Font scale in pixels
    pub scale: f32,
    /// Position of the lower-left corner
    pub pos: [f32; 2],
    /// Resource key for the uploaded texture
    pub key: &'a str,
    /// Screen dimensions for converting glyph metrics to NDC
    pub screen_size: [f32; 2],
    /// Color of the rendered text
    pub color: [f32; 4],
    /// Render bold text
    pub bold: bool,
    /// Render italic text
    pub italic: bool,
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
        bold: bool,
    ) -> Result<Self, GPUError> {
        let font = renderer.font();
        let weight = if bold { 1.1 } else { 1.0 };
        let scale = Scale::uniform(scale * weight);
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

/// Text mesh that can be updated at runtime.
pub struct DynamicText {
    vertex_buffer: Handle<Buffer>,
    index_buffer: Handle<Buffer>,
    atlas: TextAtlas,
    pub vertex_count: usize,
    pub index_count: usize,
    pub max_chars: usize,
    pub texture_key: String,
    tex_index: u32,
    color: [f32; 4],
    scale: f32,
    screen_size: [f32; 2],
    italic: bool,
}

impl TextRenderable for DynamicText {
    fn vertex_buffer(&self) -> Handle<Buffer> {
        self.vertex_buffer
    }

    fn index_buffer(&self) -> Option<Handle<Buffer>> {
        Some(self.index_buffer)
    }

    fn index_count(&self) -> usize {
        self.index_count
    }
}

impl DynamicText {
    /// Allocate buffers for up to `max_chars` worth of text.
    pub fn new(
        ctx: &mut Context,
        renderer: &mut TextRenderer2D,
        res: &mut ResourceManager,
        info: DynamicTextCreateInfo<'_>,
    ) -> Result<Self, GPUError> {
        let vertex_bytes = (info.max_chars * 4 * std::mem::size_of::<Vertex>()) as u32;
        let index_bytes = (info.max_chars * 6 * std::mem::size_of::<u32>()) as u32;

        let vertex_buffer = ctx.make_buffer(&BufferInfo {
            debug_name: "dynamic_text_vertex",
            byte_size: vertex_bytes,
            visibility: MemoryVisibility::CpuAndGpu,
            usage: BufferUsage::VERTEX,
            initial_data: None,
        })?;

        let index_buffer = ctx.make_buffer(&BufferInfo {
            debug_name: "dynamic_text_index",
            byte_size: index_bytes,
            visibility: MemoryVisibility::CpuAndGpu,
            usage: BufferUsage::INDEX,
            initial_data: None,
        })?;

        let atlas = TextAtlas::new(ctx, res, renderer, info.key, info.scale, info.bold)?;
        let idx = atlas.index;
        let mut dynamic = Self {
            vertex_buffer,
            index_buffer,
            atlas,
            vertex_count: 0,
            index_count: 0,
            max_chars: info.max_chars,
            texture_key: info.key.into(),
            tex_index: idx,
            color: info.color,
            scale: info.scale,
            screen_size: info.screen_size,
            italic: info.italic,
        };
        dynamic.update_text(ctx, res, renderer, info.text, info.scale, info.pos)?;
        Ok(dynamic)
    }

    /// Update the string contents. Fails if the text exceeds the allocated size.
    pub fn update_text(
        &mut self,
        ctx: &mut Context,
        _res: &mut ResourceManager,
        _renderer: &TextRenderer2D,
        text: &str,
        _scale: f32,
        pos: [f32; 2],
    ) -> Result<(), GPUError> {
        assert!(text.len() <= self.max_chars);
        if text.is_empty() {
            self.vertex_count = 0;
            self.index_count = 0;
            return Ok(());
        }
        let mut verts = Vec::with_capacity(text.len() * 4);
        let mut inds = Vec::with_capacity(text.len() * 6);
        let mut cursor = pos[0];
        let sx = self.screen_size[0];
        let sy = self.screen_size[1];
        for ch in text.chars() {
            if let Some(g) = self.atlas.glyphs.get(&ch) {
                let base = verts.len() as u32;
                let adv = 2.0 * g.advance / sx;
                let x0 = cursor;
                let x1 = cursor + adv;
                let y0 = pos[1] - 2.0 * self.atlas.line_height / sy;
                let y1 = pos[1];
                let shear = if self.italic { 0.25 * 2.0 * self.atlas.line_height / sy } else { 0.0 };
                let c = self.color;
                let t = [1.0, 0.0, 0.0, self.tex_index as f32];
                verts.push(Vertex { position: [x0, y0, 0.0], normal: [0.0; 3], tangent: t, uv: [g.uv_min[0], g.uv_max[1]], color: c });
                verts.push(Vertex { position: [x1, y0, 0.0], normal: [0.0; 3], tangent: t, uv: [g.uv_max[0], g.uv_max[1]], color: c });
                verts.push(Vertex { position: [x1 + shear, y1, 0.0], normal: [0.0; 3], tangent: t, uv: [g.uv_max[0], g.uv_min[1]], color: c });
                verts.push(Vertex { position: [x0 + shear, y1, 0.0], normal: [0.0; 3], tangent: t, uv: [g.uv_min[0], g.uv_min[1]], color: c });
                inds.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
                cursor += adv;
            }
        }
        let vert_bytes: &[u8] = bytemuck::cast_slice(&verts);
        let slice = ctx.map_buffer_mut(self.vertex_buffer)?;
        slice[..vert_bytes.len()].copy_from_slice(vert_bytes);
        ctx.unmap_buffer(self.vertex_buffer)?;

        let idx_bytes: &[u8] = bytemuck::cast_slice(&inds);
        let slice = ctx.map_buffer_mut(self.index_buffer)?;
        slice[..idx_bytes.len()].copy_from_slice(idx_bytes);
        ctx.unmap_buffer(self.index_buffer)?;

        self.vertex_count = verts.len();
        self.index_count = inds.len();
        Ok(())
    }

    /// GPU handle for the vertex buffer slice.
    pub fn vertex_buffer(&self) -> Handle<Buffer> {
        self.vertex_buffer
    }

    /// GPU handle for the index buffer slice.
    pub fn index_buffer(&self) -> Handle<Buffer> {
        self.index_buffer
    }

    /// Free GPU resources associated with this text.
    pub fn destroy(self, ctx: &mut Context) {
        ctx.destroy_buffer(self.vertex_buffer);
        ctx.destroy_buffer(self.index_buffer);
    }
}

