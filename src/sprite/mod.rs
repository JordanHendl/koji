use bytemuck::{Pod, Zeroable};
use dashi::*;
use dashi::utils::Handle;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

#[derive(Clone)]
pub struct Sprite {
    pub vertices: Vec<SpriteVertex>,
    pub indices: Option<Vec<u32>>,
    pub vertex_buffer: Option<Handle<Buffer>>,
    pub index_buffer: Option<Handle<Buffer>>,
    pub index_count: usize,
}

impl Sprite {
    pub fn upload(&mut self, ctx: &mut Context) -> Result<(), GPUError> {
        let bytes: &[u8] = bytemuck::cast_slice(&self.vertices);
        self.vertex_buffer = Some(ctx.make_buffer(&BufferInfo {
            debug_name: "sprite_vertex_buffer",
            byte_size: bytes.len() as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::VERTEX,
            initial_data: Some(bytes),
        })?);
        if let Some(ref idx) = self.indices {
            let idx_bytes: &[u8] = bytemuck::cast_slice(idx);
            self.index_buffer = Some(ctx.make_buffer(&BufferInfo {
                debug_name: "sprite_index_buffer",
                byte_size: idx_bytes.len() as u32,
                visibility: MemoryVisibility::Gpu,
                usage: BufferUsage::INDEX,
                initial_data: Some(idx_bytes),
            })?);
            self.index_count = idx.len();
        } else {
            self.index_count = self.vertices.len();
        }
        Ok(())
    }
}

pub mod renderer;
