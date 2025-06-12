//! Types of drawables that can be registered with the renderer.
//! These may expand to support more mesh/material types and instancing.

//!
//! Skeletal meshes expose [`SkeletalMesh::update_bones`] to upload
//! bone matrices each frame. The [`Renderer`](crate::renderer::Renderer)
//! provides a helper to call this on registered meshes.
use dashi::{utils::Handle, *};
use glam::Mat4;
use crate::animation::{Animator, Skeleton};

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
    pub material_id: String,
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

    /// Upload updated bone matrices to the GPU.
    pub fn update_bones(&self, ctx: &mut Context, matrices: &[Mat4]) -> Result<(), GPUError> {
        let buffer = self
            .bone_buffer
            .expect("Skeletal mesh not uploaded or bone buffer missing");
        let bytes: &[u8] = bytemuck::cast_slice(matrices);
        let slice = ctx.map_buffer_mut(buffer)?;
        slice[..bytes.len()].copy_from_slice(bytes);
        ctx.unmap_buffer(buffer)?;
        Ok(())
    }
}

/// Skeletal mesh data with optional GPU resources.
#[derive(Debug, Clone)]
pub struct SkeletalMesh {
    pub material_id: String,
    pub vertices: Vec<SkeletalVertex>,
    pub indices: Option<Vec<u32>>,
    pub vertex_buffer: Option<Handle<Buffer>>,
    pub index_buffer: Option<Handle<Buffer>>,
    pub index_count: usize,
    pub skeleton: Skeleton,
    pub bone_buffer: Option<Handle<Buffer>>,
}

/// A runtime instance of a skeletal mesh with its own animator and GPU buffer.
pub struct SkeletalInstance {
    pub animator: Animator,
    pub bone_buffer: Handle<Buffer>,
}

impl SkeletalInstance {
    /// Create a new instance with GPU storage for bone matrices.
    pub fn new(ctx: &mut Context, animator: Animator) -> Result<Self, GPUError> {
        let bone_buffer = ctx.make_buffer(&BufferInfo {
            debug_name: "skel_instance_bones",
            byte_size: (animator.skeleton.bone_count() * std::mem::size_of::<Mat4>()) as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::STORAGE,
            initial_data: None,
        })?;
        Ok(Self { animator, bone_buffer })
    }

    /// Upload the animator's matrices to the GPU buffer.
    pub fn update_gpu(&self, ctx: &mut Context) -> Result<(), GPUError> {
        let bytes: &[u8] = bytemuck::cast_slice(&self.animator.matrices);
        let slice = ctx.map_buffer_mut(self.bone_buffer)?;
        slice[..bytes.len()].copy_from_slice(bytes);
        ctx.unmap_buffer(self.bone_buffer)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use dashi::*;
    use serial_test::serial;
    use crate::animation::Bone;

    fn make_ctx() -> Context {
        Context::headless(&ContextInfo::default()).unwrap()
    }

    fn simple_vertex() -> Vertex {
        Vertex {
            position: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    fn simple_skel_vertex() -> SkeletalVertex {
        SkeletalVertex {
            position: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
            joint_indices: [0, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    #[test]
    #[serial]
    #[cfg_attr(not(feature = "gpu_tests"), ignore)]
    fn upload_static_mesh_sets_buffers_valid() {
        let mut ctx = make_ctx();
        let mut mesh = StaticMesh {
            material_id: "test".into(),
            vertices: vec![simple_vertex(), simple_vertex(), simple_vertex()],
            indices: Some(vec![0, 1, 2]),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        };
        mesh.upload(&mut ctx).unwrap();
        assert!(mesh.vertex_buffer.unwrap().valid());
        assert!(mesh.index_buffer.unwrap().valid());
        assert_eq!(mesh.index_count, 3);
        ctx.destroy();
    }

    #[test]
    #[serial]
    #[cfg_attr(not(feature = "gpu_tests"), ignore)]
    fn upload_skeletal_mesh_creates_bone_buffer() {
        let mut ctx = make_ctx();
        let skeleton = Skeleton { bones: vec![Bone::default(); 2] };
        let mut mesh = SkeletalMesh {
            material_id: "test".into(),
            vertices: vec![simple_skel_vertex()],
            indices: None,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            skeleton,
            bone_buffer: None,
        };
        mesh.upload(&mut ctx).unwrap();
        let bone_buf = mesh.bone_buffer.expect("bone buffer");
        assert!(bone_buf.valid());
        let slice = ctx.map_buffer::<u8>(bone_buf).unwrap();
        assert_eq!(slice.len(), mesh.skeleton.bone_count() * std::mem::size_of::<Mat4>());
        ctx.unmap_buffer(bone_buf).unwrap();
        ctx.destroy();
    }

    #[test]
    #[serial]
    #[cfg_attr(not(feature = "gpu_tests"), ignore)]
    fn update_bones_writes_matrices() {
        let mut ctx = make_ctx();
        let skeleton = Skeleton { bones: vec![Bone::default(); 2] };
        let mut mesh = SkeletalMesh {
            material_id: "test".into(),
            vertices: vec![simple_skel_vertex()],
            indices: None,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            skeleton,
            bone_buffer: None,
        };
        mesh.upload(&mut ctx).unwrap();
        let mats = vec![Mat4::IDENTITY, Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0))];
        mesh.update_bones(&mut ctx, &mats).unwrap();
        let bone_buf = mesh.bone_buffer.unwrap();
        let mapped = ctx.map_buffer::<Mat4>(bone_buf).unwrap();
        assert_eq!(mapped[0], mats[0]);
        assert_eq!(mapped[1], mats[1]);
        ctx.unmap_buffer(bone_buf).unwrap();
        ctx.destroy();
    }

    #[test]
    #[serial]
    #[cfg_attr(not(feature = "gpu_tests"), ignore)]
    #[should_panic(expected = "Skeletal mesh not uploaded or bone buffer missing")]
    fn update_bones_panics_without_upload() {
        let mut ctx = make_ctx();
        let skeleton = Skeleton { bones: vec![Bone::default()] };
        let mesh = SkeletalMesh {
            material_id: "test".into(),
            vertices: vec![simple_skel_vertex()],
            indices: None,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            skeleton,
            bone_buffer: None,
        };
        let mats = vec![Mat4::IDENTITY];
        mesh.update_bones(&mut ctx, &mats).unwrap();
    }
}
