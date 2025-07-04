use crate::renderer::Vertex;
use crate::text::{TextRenderer2D, TextRenderable};
use crate::utils::ResourceManager;
use dashi::utils::Handle;
use dashi::*;

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
}

/// Text mesh that can be updated at runtime.
pub struct DynamicText {
    vertex_buffer: Handle<Buffer>,
    index_buffer: Handle<Buffer>,
    pub vertex_count: usize,
    pub index_count: usize,
    pub max_chars: usize,
    pub texture_key: String,
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
        renderer: &TextRenderer2D,
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

        let mut dynamic = Self {
            vertex_buffer,
            index_buffer,
            vertex_count: 0,
            index_count: 0,
            max_chars: info.max_chars,
            texture_key: info.key.into(),
        };
        dynamic.update_text(ctx, res, renderer, info.text, info.scale, info.pos)?;
        Ok(dynamic)
    }

    /// Update the string contents. Fails if the text exceeds the allocated size.
    pub fn update_text(
        &mut self,
        ctx: &mut Context,
        res: &mut ResourceManager,
        renderer: &TextRenderer2D,
        text: &str,
        scale: f32,
        pos: [f32; 2],
    ) -> Result<(), GPUError> {
        assert!(text.len() <= self.max_chars);
        if text.is_empty() {
            self.vertex_count = 0;
            self.index_count = 0;
            return Ok(());
        }
        let dim = renderer.upload_text_texture(ctx, res, &self.texture_key, text, scale)?;
        let mesh = renderer.make_quad(dim, pos);
        let vert_bytes: &[u8] = bytemuck::cast_slice(&mesh.vertices);
        let slice = ctx.map_buffer_mut(self.vertex_buffer)?;
        slice[..vert_bytes.len()].copy_from_slice(vert_bytes);
        ctx.unmap_buffer(self.vertex_buffer)?;

        let idx = mesh.indices.as_ref().expect("indices");
        let idx_bytes: &[u8] = bytemuck::cast_slice(idx);
        let slice = ctx.map_buffer_mut(self.index_buffer)?;
        slice[..idx_bytes.len()].copy_from_slice(idx_bytes);
        ctx.unmap_buffer(self.index_buffer)?;

        self.vertex_count = mesh.vertices.len();
        self.index_count = idx.len();
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

