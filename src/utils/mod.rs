use std::collections::HashMap;

use dashi::*;
use utils::Handle;

mod allocator;
mod resource_list;
use allocator::*;
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

#[derive(Debug)]
pub struct DHObject {
    pub handle: Handle<Buffer>,
    pub offset: u64,
    pub size: u64,
}

impl DHObject {
    pub fn new<T: Copy>(
        ctx: &mut Context,
        allocator: &mut GpuAllocator,
        value: T,
    ) -> Result<Self, GPUError> {
        let size = std::mem::size_of::<T>() as u64;
        let alloc = allocator.allocate(size).ok_or(GPUError::LibraryError())?;
        let mut slice = unsafe { ctx.map_buffer_mut(alloc.buffer)? };
        let bytes = unsafe {
            std::slice::from_raw_parts(&value as *const T as *const u8, std::mem::size_of::<T>())
        };
        slice[(alloc.offset as usize)..(alloc.offset as usize + bytes.len())]
            .copy_from_slice(bytes);
        Ok(Self {
            handle: alloc.buffer,
            offset: alloc.offset,
            size: alloc.size,
        })
    }
}

#[derive(Default)]
pub struct ResourceManager {
    pub allocator: GpuAllocator,
    pub textures: ResourceList<Texture>,
    pub buffers: ResourceList<DHObject>,
    texture_keys: HashMap<String, Handle<Texture>>,
    buffer_keys: HashMap<String, Handle<DHObject>>,
}

impl ResourceManager {
    pub fn new(ctx: &mut Context, byte_size: u64) -> Result<Self, GPUError> {
        let allocator = GpuAllocator::new(ctx, byte_size, BufferUsage::STORAGE, 256)?;
        Ok(Self {
            allocator,
            textures: Default::default(),
            buffers: Default::default(),
            texture_keys: Default::default(),
            buffer_keys: Default::default(),
        })
    }

    pub fn register_texture(
        &mut self,
        key: impl Into<String>,
        info: &TextureInfo,
    ) -> Handle<Texture> {
        let tex = Texture {
            handle: info.image,
            sampler: info.sampler,
            dim: info.dim,
            view: info.view,
        };
        let handle = self.textures.push(tex);
        self.texture_keys.insert(key.into(), handle);
        handle
    }

    pub fn unregister_texture(&mut self, key: &str) {
        if let Some(handle) = self.texture_keys.remove(key) {
            self.textures.release(handle);
        }
    }

    pub fn register_buffer<T: Copy>(
        &mut self,
        key: impl Into<String>,
        ctx: &mut Context,
        data: T,
    ) -> Handle<DHObject> {
        let dh = DHObject::new(ctx, &mut self.allocator, data).unwrap();
        let handle = self.buffers.push(dh);
        self.buffer_keys.insert(key.into(), handle);
        handle
    }

    pub fn get_texture(&self, key: &str) -> Option<&Texture> {
        self.texture_keys
            .get(key)
            .map(|h| self.textures.get_ref(*h))
    }

    pub fn allocator(&self) -> &GpuAllocator {
        &self.allocator
    }

    pub fn get_buffer(&self, key: &str) -> Option<&DHObject> {
        self.buffer_keys.get(key).map(|h| self.buffers.get_ref(*h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashi::{gpu, DeviceFilter, DeviceSelector};
    use serial_test::serial;

    fn init_ctx() -> gpu::Context {
        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        gpu::Context::new(&ContextInfo { device }).unwrap()
    }

    #[test]
    #[serial]
    fn register_and_lookup_texture() {
        let mut ctx = init_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024 * 1024).unwrap();

        let texture = TextureInfo {
            image: Handle::default(),
            view: Handle::default(),
            sampler: Handle::default(),
            dim: [128, 128],
        };

        let handle = manager.register_texture("my_tex", &texture);
        let found = manager.get_texture("my_tex");

        assert!(found.is_some());
        assert_eq!(found.unwrap().dim, [128, 128]);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn unregister_resources() {
        let mut ctx = init_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024 * 1024).unwrap();

        let texture = TextureInfo {
            image: Handle::default(),
            view: Handle::default(),
            sampler: Handle::default(),
            dim: [256, 256],
        };

        manager.register_texture("my_tex", &texture);
        assert!(manager.get_texture("my_tex").is_some());

        manager.unregister_texture("my_tex");
        assert!(manager.get_texture("my_tex").is_none());

        ctx.destroy();
    }
}
