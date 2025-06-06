use crate::utils::{CombinedTextureSampler, ResourceBuffer, ResourceList, ResourceManager, Texture, DHObject};
use dashi::{Context, Image, ImageView, Sampler};
use dashi::utils::Handle;
use std::sync::Arc;

/// Container for textures and material buffers used with bindless descriptors.
pub struct BindlessData {
    textures: ResourceList<CombinedTextureSampler>,
    materials: ResourceList<ResourceBuffer>,
}

impl BindlessData {
    /// Create an empty collection.
    pub fn new() -> Self {
        Self { textures: ResourceList::default(), materials: ResourceList::default() }
    }

    /// Add a texture/sampler pair. Returns the index in the bindless array.
    pub fn add_texture(&mut self, img: Handle<Image>, view: Handle<ImageView>, sampler: Handle<Sampler>, dim: [u32; 2]) -> u32 {
        let tex = CombinedTextureSampler { texture: Texture { handle: img, view, dim }, sampler };
        self.textures.push(tex);
        (self.textures.len() - 1) as u32
    }

    /// Add an arbitrary material struct as a GPU buffer.
    pub fn add_material<T: bytemuck::Pod>(&mut self, ctx: &mut Context, res: &mut ResourceManager, data: T) -> u32 {
        let dh = DHObject::new(ctx, &mut res.allocator, data).unwrap();
        self.materials.push(ResourceBuffer::from(dh));
        (self.materials.len() - 1) as u32
    }

    /// Register the arrays with a [`ResourceManager`] so shaders can access them.
    pub fn register(self, res: &mut ResourceManager) {
        res.register_combined_texture_array("bindless_textures", Arc::new(self.textures));
        res.register_buffer_array("bindless_materials", Arc::new(self.materials));
    }
}


