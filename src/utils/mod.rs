use dashi::*;
use utils::Handle;
mod resource_list;
mod allocator;
use resource_list::*;
pub struct TextureInfo {
    pub image: Handle<Image>,
    pub view: Handle<ImageView>,
    pub sampler: Handle<Sampler>,
    pub dim: [u32; 2],
}

#[allow(dead_code)]
pub struct Texture {
    pub(crate) handle: Handle<Image>,
    pub(crate) view: Handle<ImageView>,
    pub(crate) sampler: Handle<Sampler>,
    pub(crate) dim: [u32; 2],
}

#[derive(Default)]
pub struct ResourceManager {
    pub textures: ResourceList<Texture>,
}

struct ResourceRegistry {
    res: ResourceManager,
}

impl ResourceRegistry {
    pub fn new(ctx: &mut Context) -> Self {
        todo!()
    }

    pub fn register_texture(&mut self, info: &TextureInfo) -> Handle<Texture> {
        return self.res.textures.push(Texture {
            handle: info.image,
            sampler: info.sampler,
            dim: info.dim,
            view: info.view,
        });
    }

    pub fn unregister_texture(&mut self, h: Handle<Texture>) {
        self.res.textures.release(h);
    }
}
