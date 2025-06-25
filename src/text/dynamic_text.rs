use crate::renderer::Vertex;
use crate::text::TextRenderer2D;
use crate::utils::{GpuAllocator, ResourceManager};
use crate::utils::allocator::Allocation;
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
    vertex_alloc: Allocation,
    index_alloc: Allocation,
    allocator: GpuAllocator,
    pub vertex_count: usize,
    pub index_count: usize,
    pub max_chars: usize,
    pub texture_key: String,
}

impl DynamicText {
    /// Allocate buffers for up to `max_chars` worth of text.
    pub fn new(
        ctx: &mut Context,
        renderer: &TextRenderer2D,
        res: &mut ResourceManager,
        info: DynamicTextCreateInfo<'_>,
    ) -> Result<Self, GPUError> {
        let vertex_bytes = (info.max_chars * 4 * std::mem::size_of::<Vertex>()) as u64;
        let index_bytes = (info.max_chars * 6 * std::mem::size_of::<u32>()) as u64;
        let mut allocator = GpuAllocator::new(ctx, vertex_bytes + index_bytes, BufferUsage::ALL, 256)?;
        let vertex_alloc = allocator
            .allocate(vertex_bytes)
            .ok_or(GPUError::LibraryError())?;
        let index_alloc = allocator
            .allocate(index_bytes)
            .ok_or(GPUError::LibraryError())?;

        let mut dynamic = Self {
            vertex_alloc,
            index_alloc,
            allocator,
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
        let dim = renderer.upload_text_texture(ctx, res, &self.texture_key, text, scale);
        let mesh = renderer.make_quad(dim, pos);
        let vert_bytes: &[u8] = bytemuck::cast_slice(&mesh.vertices);
        assert!(vert_bytes.len() as u64 <= self.vertex_alloc.size);
        let slice = ctx.map_buffer_mut(self.vertex_alloc.buffer)?;
        let start = self.vertex_alloc.offset as usize;
        slice[start..start + vert_bytes.len()].copy_from_slice(vert_bytes);
        ctx.unmap_buffer(self.vertex_alloc.buffer)?;

        let idx = mesh.indices.as_ref().expect("indices");
        let idx_bytes: &[u8] = bytemuck::cast_slice(idx);
        assert!(idx_bytes.len() as u64 <= self.index_alloc.size);
        let slice = ctx.map_buffer_mut(self.index_alloc.buffer)?;
        let start = self.index_alloc.offset as usize;
        slice[start..start + idx_bytes.len()].copy_from_slice(idx_bytes);
        ctx.unmap_buffer(self.index_alloc.buffer)?;

        self.vertex_count = mesh.vertices.len();
        self.index_count = idx.len();
        Ok(())
    }

    /// GPU handle for the vertex buffer slice.
    pub fn vertex_buffer(&self) -> Handle<Buffer> {
        self.vertex_alloc.buffer
    }

    /// GPU handle for the index buffer slice.
    pub fn index_buffer(&self) -> Handle<Buffer> {
        self.index_alloc.buffer
    }

    /// Free GPU resources associated with this text.
    pub fn destroy(self, ctx: &mut Context) {
        ctx.destroy_buffer(self.vertex_alloc.buffer);
        ctx.destroy_buffer(self.index_alloc.buffer);
        self.allocator.destroy(ctx);
    }
}

