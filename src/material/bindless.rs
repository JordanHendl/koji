use crate::utils::{CombinedTextureSampler, ResourceBuffer, ResourceList, ResourceManager, Texture, DHObject};
use dashi::{Context, Image, ImageView, Sampler};
use dashi::utils::Handle;
use std::sync::{Arc, Mutex};

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
        res.register_buffer_array("bindless_materials", Arc::new(Mutex::new(self.materials)));
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use dashi::gpu;
    use dashi::*;
    use serial_test::serial;
    use crate::ResourceBinding;

    fn setup_ctx() -> gpu::Context {
        gpu::Context::headless(&Default::default()).unwrap()
    }

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Dummy(u32);

    #[test]
    #[serial]
    fn new_is_empty() {
        let data = BindlessData::new();
        assert_eq!(data.textures.len(), 0);
        assert_eq!(data.materials.len(), 0);
    }

    #[test]
    #[serial]
    fn add_texture_stores_and_returns_index() {
        let mut ctx = setup_ctx();
        let mut data = BindlessData::new();
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
        let img1 = ctx.make_image(&ImageInfo::default()).unwrap();
        let view1 = ctx
            .make_image_view(&ImageViewInfo { img: img1, ..Default::default() })
            .unwrap();
        let img2 = ctx.make_image(&ImageInfo::default()).unwrap();
        let view2 = ctx
            .make_image_view(&ImageViewInfo { img: img2, ..Default::default() })
            .unwrap();

        let idx0 = data.add_texture(img1, view1, sampler, [1, 1]);
        let idx1 = data.add_texture(img2, view2, sampler, [1, 1]);

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);

        let h0 = data.textures.entries[idx0 as usize];
        let stored0 = data.textures.get_ref(h0);
        assert_eq!(stored0.texture.handle, img1);
        assert_eq!(stored0.texture.view, view1);
        assert_eq!(stored0.sampler, sampler);

        ctx.destroy();
    }

    #[test]
    #[serial]
    fn add_material_uploads_and_stores() {
        let mut ctx = setup_ctx();
        let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();
        let mut data = BindlessData::new();

        let idx = data.add_material(&mut ctx, &mut res, Dummy(42));
        assert_eq!(idx, 0);

        let h = data.materials.entries[idx as usize];
        let buf = data.materials.get_ref(h);
        let slice = ctx.map_buffer::<u8>(buf.handle).unwrap();
        let bytes = &slice[buf.offset as usize..buf.offset as usize + std::mem::size_of::<Dummy>()];
        let val = *bytemuck::from_bytes::<Dummy>(bytes);
        ctx.unmap_buffer(buf.handle).unwrap();
        assert_eq!(val.0, 42);

        res.destroy(&mut ctx);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_inserts_bindings() {
        let mut ctx = setup_ctx();
        let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
        let img = ctx.make_image(&ImageInfo::default()).unwrap();
        let view = ctx
            .make_image_view(&ImageViewInfo { img, ..Default::default() })
            .unwrap();

        let mut data = BindlessData::new();
        data.add_texture(img, view, sampler, [1, 1]);
        data.add_material(&mut ctx, &mut res, Dummy(7));
        data.register(&mut res);

        match res.get("bindless_textures") {
            Some(ResourceBinding::CombinedTextureArray(arr)) => {
                assert_eq!(arr.len(), 1);
            }
            _ => panic!("expected combined texture array"),
        }
        match res.get("bindless_materials") {
            Some(ResourceBinding::BufferArray(arr)) => {
                assert_eq!(arr.lock().unwrap().len(), 1);
            }
            _ => panic!("expected buffer array"),
        }

        res.destroy(&mut ctx);
        ctx.destroy();
    }
}
