//! Types of drawables that can be registered with the renderer.
//! These may expand to support more mesh/material types and instancing.
use dashi::{utils::Handle, *};
use glam::Mat4;
use crate::animation::Skeleton;

use bytemuck::{Pod, Zeroable};

/// Vertex type for PBR rendering (static meshes)
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

/// Vertex type for skeletal meshes (adds skinning info)
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct SkeletalVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub joint_indices: [u16; 4],
    pub joint_weights: [f32; 4],
}

pub struct StaticMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Option<Vec<u32>>,
    pub vertex_buffer: Option<Handle<Buffer>>,
    pub index_buffer: Option<Handle<Buffer>>,
    pub index_count: usize,
}

impl StaticMesh {
    pub fn upload(&mut self, ctx: &mut Context) -> Result<(), GPUError> {
        let bytes: &[u8] = bytemuck::cast_slice(&self.vertices);
        self.vertex_buffer = Some(ctx.make_buffer(&BufferInfo {
            debug_name: "mesh_vertex_buffer",
            byte_size: bytes.len() as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::VERTEX,
            initial_data: Some(bytes),
        })?);
        if let Some(ref idx) = self.indices {
            let idx_bytes: &[u8] = bytemuck::cast_slice(idx);
            self.index_buffer = Some(ctx.make_buffer(&BufferInfo {
                debug_name: "mesh_index_buffer",
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

impl SkeletalMesh {
    pub fn upload(&mut self, ctx: &mut Context) -> Result<(), GPUError> {
        let bytes: &[u8] = bytemuck::cast_slice(&self.vertices);
        self.vertex_buffer = Some(ctx.make_buffer(&BufferInfo {
            debug_name: "skel_mesh_vertex_buffer",
            byte_size: bytes.len() as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::VERTEX,
            initial_data: Some(bytes),
        })?);
        if let Some(ref idx) = self.indices {
            let idx_bytes: &[u8] = bytemuck::cast_slice(idx);
            self.index_buffer = Some(ctx.make_buffer(&BufferInfo {
                debug_name: "skel_mesh_index_buffer",
                byte_size: idx_bytes.len() as u32,
                visibility: MemoryVisibility::Gpu,
                usage: BufferUsage::INDEX,
                initial_data: Some(idx_bytes),
            })?);
            self.index_count = idx.len();
        } else {
            self.index_count = self.vertices.len();
        }
        self.bone_buffer = Some(ctx.make_buffer(&BufferInfo {
            debug_name: "skel_bone_buffer",
            byte_size: (self.skeleton.bone_count() * std::mem::size_of::<Mat4>()) as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::STORAGE,
            initial_data: None,
        })?);
        Ok(())
    }
}

/// Placeholder for skeletal mesh (not implemented in this pass).
#[derive(Debug, Clone)]
pub struct SkeletalMesh {
    pub vertices: Vec<SkeletalVertex>,
    pub indices: Option<Vec<u32>>,
    pub vertex_buffer: Option<Handle<Buffer>>,
    pub index_buffer: Option<Handle<Buffer>>,
    pub index_count: usize,
    pub skeleton: Skeleton,
    pub bone_buffer: Option<Handle<Buffer>>,
}
