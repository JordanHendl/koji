use std::{collections::HashMap, sync::{Arc, Mutex}};

use dashi::*;
use dashi::BufferUsage as DashiBufferUsage;
use utils::Handle;

#[derive(Clone, Copy)]
struct BufferUsage(u8);

impl BufferUsage {
    const STORAGE: Self = Self(1 << 0);
    const UNIFORM: Self = Self(1 << 1);
}

impl std::ops::BitOr for BufferUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        BufferUsage(self.0 | rhs.0)
    }
}

impl From<BufferUsage> for DashiBufferUsage {
    fn from(value: BufferUsage) -> Self {
        match value.0 {
            x if x == BufferUsage::STORAGE.0 => DashiBufferUsage::STORAGE,
            x if x == BufferUsage::UNIFORM.0 => DashiBufferUsage::UNIFORM,
            x if x == (BufferUsage::STORAGE.0 | BufferUsage::UNIFORM.0) => DashiBufferUsage::ALL,
            _ => DashiBufferUsage::ALL,
        }
    }
}

pub mod allocator;
pub mod resource_list;
pub use allocator::*;
pub use resource_list::*;

pub struct TextureInfo {
    pub image: Handle<Image>,
    pub view: Handle<ImageView>,
    pub sampler: Handle<Sampler>,
    pub dim: [u32; 2],
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
        const MIN_BYTES: u64 = 32;
        let mut size = std::mem::size_of::<T>() as u64;
        size = size.max(MIN_BYTES);
        let alloc = allocator.allocate(size).ok_or(GPUError::LibraryError())?;
        let slice = ctx.map_buffer_mut(alloc.buffer)?;
        let bytes = unsafe {
            std::slice::from_raw_parts(&value as *const T as *const u8, std::mem::size_of::<T>())
        };
        slice[..bytes.len()].copy_from_slice(bytes);

        let _ = ctx.unmap_buffer(alloc.buffer);
        Ok(Self {
            handle: alloc.buffer,
            offset: alloc.offset,
            size: alloc.size,
        })
    }

    pub fn new_from_bytes(
        ctx: &mut Context,
        allocator: &mut GpuAllocator,
        value: &[u8],
    ) -> Result<Self, GPUError> {
        let size = value.len();
        let alloc = allocator.allocate(size as u64).ok_or(GPUError::LibraryError())?;
        let slice =  ctx.map_buffer_mut(alloc.buffer)?;
        slice[..value.len()].copy_from_slice(value);

        let _ = ctx.unmap_buffer(alloc.buffer);
        Ok(Self {
            handle: alloc.buffer,
            offset: alloc.offset,
            size: alloc.size,
        })
    }

}

#[derive(Clone, Copy, Debug)]
pub struct Texture {
    pub handle: Handle<Image>,
    pub view: Handle<ImageView>,
    pub dim: [u32; 2],
}

#[derive(Clone)]
pub struct CombinedTextureSampler {
    pub texture: Texture,
    pub sampler: Handle<Sampler>,
}

#[derive(Clone)]
pub struct ResourceBuffer {
    pub handle: Handle<Buffer>,
    pub offset: u64,
}

impl From<DHObject> for ResourceBuffer {
    fn from(obj: DHObject) -> Self {
        Self {
            handle: obj.handle,
            offset: obj.offset,
        }
    }
}

pub enum ResourceBinding {
    Texture(Texture),
    Uniform(Handle<Buffer>),
    Storage(Handle<Buffer>),
    TextureArray(Arc<ResourceList<Texture>>),
    CombinedTextureArray(Arc<ResourceList<CombinedTextureSampler>>),
    BufferArray(Arc<Mutex<ResourceList<ResourceBuffer>>>),
    CombinedImageSampler {
        texture: Texture,
        sampler: Handle<Sampler>,
    },
}

#[derive(Default)]
pub struct ResourceManager {
    pub allocator: GpuAllocator,
    pub textures: ResourceList<Texture>,
    pub buffers: ResourceList<ResourceBuffer>,
    pub bindings: HashMap<String, ResourceBinding>,
}

impl ResourceManager {
    pub fn new(ctx: &mut Context, byte_size: u64) -> Result<Self, GPUError> {
        let usage: DashiBufferUsage = (BufferUsage::STORAGE | BufferUsage::UNIFORM).into();
        let allocator = GpuAllocator::new(ctx, byte_size, usage, 256)?;
        Ok(Self {
            allocator,
            textures: Default::default(),
            buffers: Default::default(),
            bindings: Default::default(),
        })
    }
        
    pub fn destroy(mut self, ctx: &mut Context) {
        self.allocator.reset();
        self.allocator.destroy(ctx);
    }
    pub fn register_texture(
        &mut self,
        key: impl Into<String>,
        image: Handle<Image>,
        view: Handle<ImageView>,
        dim: [u32; 2],
    ) {
        let tex = Texture {
            handle: image,
            view,
            dim,
        };
        self.bindings
            .insert(key.into(), ResourceBinding::Texture(tex));
    }

    pub fn register_combined(
        &mut self,
        key: impl Into<String>,
        image: Handle<Image>,
        view: Handle<ImageView>,
        dim: [u32; 2],
        sampler: Handle<Sampler>,
    ) {
        let tex = Texture {
            handle: image,
            view,
            dim,
        };
        self.bindings.insert(
            key.into(),
            ResourceBinding::CombinedImageSampler {
                texture: tex,
                sampler,
            },
        );
    }

    pub fn register_variable_bytes(&mut self, key: impl Into<String>, ctx: &mut Context, data: &[u8]) {
        let dh = DHObject::new_from_bytes(ctx, &mut self.allocator, data).unwrap();
        let buf = ResourceBuffer::from(dh);
        self.buffers.push(buf.clone());
        self.bindings
            .insert(key.into(), ResourceBinding::Uniform(buf.handle));
    }

    pub fn register_variable<T: Copy>(&mut self, key: impl Into<String>, ctx: &mut Context, data: T) {
        let dh = DHObject::new(ctx, &mut self.allocator, data).unwrap();
        let buf = ResourceBuffer::from(dh);
        self.buffers.push(buf.clone());
        self.bindings
            .insert(key.into(), ResourceBinding::Uniform(buf.handle));
    }

    pub fn register_time_buffers(&mut self, ctx: &mut Context) {
        let time_data = [0.0f32, 0.0f32];
        let dh = DHObject::new(ctx, &mut self.allocator, time_data).unwrap();
        let buf = ResourceBuffer::from(dh);
        self.buffers.push(buf.clone());
        self.bindings
            .insert("time".into(), ResourceBinding::Uniform(buf.handle));
        self.bindings
            .insert("KOJI_time".into(), ResourceBinding::Uniform(buf.handle));
    }

      pub fn register_ubo(&mut self, key: impl Into<String>, handle: Handle<Buffer>) {
        self.bindings.insert(key.into(), ResourceBinding::Uniform(handle));
    }

    // Register an existing storage buffer
    pub fn register_storage(&mut self, key: impl Into<String>, handle: Handle<Buffer>) {
        self.bindings.insert(key.into(), ResourceBinding::Storage(handle));
    }

    pub fn register_texture_array(
        &mut self,
        key: impl Into<String>,
        array: Arc<ResourceList<Texture>>,
    ) {
        self.bindings
            .insert(key.into(), ResourceBinding::TextureArray(array));
    }

    pub fn register_combined_texture_array(
        &mut self,
        key: impl Into<String>,
        array: Arc<ResourceList<CombinedTextureSampler>>,
    ) {
        self.bindings
            .insert(key.into(), ResourceBinding::CombinedTextureArray(array));
    }

    pub fn register_buffer_array(
        &mut self,
        key: impl Into<String>,
        array: Arc<Mutex<ResourceList<ResourceBuffer>>>,
    ) {
        self.bindings
            .insert(key.into(), ResourceBinding::BufferArray(array));
    }

    // pub fn register_sampler_array(&mut self, _key: impl Into<String>, _array: Arc<ResourceList<Handle<Sampler>>>) {
    //     unimplemented!("Sampler array binding not implemented yet.");
    // }
    pub fn get(&self, key: &str) -> Option<&ResourceBinding> {
        self.bindings.get(key)
    }

    pub fn remove(&mut self, key: &str) {
        self.bindings.remove(key);
    }
}

#[cfg(all(test, feature = "gpu_tests"))]
mod tests {
    use super::*;
    use dashi::gpu;
    use serial_test::serial;
    use std::sync::Arc;

    fn setup_ctx() -> gpu::Context {
        gpu::Context::headless(&Default::default()).unwrap()
    }

    #[test]
    #[serial]
    fn register_texture_binding() {
        let mut ctx = setup_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024 * 1024).unwrap();

        let image = ctx.make_image(&ImageInfo::default()).unwrap();
        let view = ctx
            .make_image_view(&ImageViewInfo {
                img: image,
                ..Default::default()
            })
            .unwrap();

        manager.register_texture("tex", image, view, [64, 64]);
        match manager.get("tex") {
            Some(ResourceBinding::Texture(tex)) => {
                assert_eq!(tex.dim, [64, 64]);
                assert_eq!(tex.handle, image);
                assert_eq!(tex.view, view);
            }
            _ => panic!("Expected texture binding"),
        }
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_combined_binding() {
        let mut ctx = setup_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024).unwrap();

        let image = ctx.make_image(&ImageInfo::default()).unwrap();
        let view = ctx
            .make_image_view(&ImageViewInfo {
                img: image,
                ..Default::default()
            })
            .unwrap();
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();

        manager.register_combined("combo", image, view, [32, 32], sampler);
        match manager.get("combo") {
            Some(ResourceBinding::CombinedImageSampler {
                texture,
                sampler: s,
            }) => {
                assert_eq!(texture.handle, image);
                assert_eq!(texture.view, view);
                assert_eq!(texture.dim, [32, 32]);
                assert_eq!(*s, sampler);
            }
            _ => panic!("Expected combined sampler"),
        }
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_texture_array_binding() {
        let manager = &mut ResourceManager::default();
        let array = Arc::new(ResourceList::<Texture>::default());
        manager.register_texture_array("array_tex", array.clone());

        match manager.get("array_tex") {
            Some(ResourceBinding::TextureArray(arr)) => {
                assert!(Arc::ptr_eq(arr, &array));
            }
            _ => panic!("Expected texture array binding"),
        }
    }

    #[test]
    #[serial]
    fn register_buffer_array_binding() {
        let manager = &mut ResourceManager::default();
        let array = Arc::new(Mutex::new(ResourceList::<ResourceBuffer>::default()));
        manager.register_buffer_array("array_buf", array.clone());

        match manager.get("array_buf") {
            Some(ResourceBinding::BufferArray(arr)) => {
                assert!(Arc::ptr_eq(arr, &array));
            }
            _ => panic!("Expected buffer array binding"),
        }
    }

    #[test]
    #[serial]
    fn register_variable_bytes_binding() {
        let mut ctx = setup_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024).unwrap();

        manager.register_variable_bytes("var_bytes", &mut ctx, &[1u8, 2, 3, 4]);

        let handle = match manager.get("var_bytes") {
            Some(ResourceBinding::Uniform(h)) => *h,
            _ => panic!("Expected uniform binding"),
        };

        assert_eq!(manager.buffers.entries.len(), 1);
        let buf_handle = manager
            .buffers
            .get_ref(manager.buffers.entries[0])
            .handle;
        assert_eq!(buf_handle, handle);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_variable_binding() {
        let mut ctx = setup_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024).unwrap();

        manager.register_variable("var", &mut ctx, 55u32);

        let handle = match manager.get("var") {
            Some(ResourceBinding::Uniform(h)) => *h,
            _ => panic!("Expected uniform binding"),
        };

        assert_eq!(manager.buffers.entries.len(), 1);
        let stored_handle = manager
            .buffers
            .get_ref(manager.buffers.entries[0])
            .handle;
        assert_eq!(stored_handle, handle);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_time_buffers_binding() {
        let mut ctx = setup_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024).unwrap();

        manager.register_time_buffers(&mut ctx);

        let handle = match manager.get("time") {
            Some(ResourceBinding::Uniform(h)) => *h,
            _ => panic!("Expected uniform binding"),
        };

        assert_eq!(manager.buffers.entries.len(), 1);
        let stored_handle = manager
            .buffers
            .get_ref(manager.buffers.entries[0])
            .handle;
        assert_eq!(stored_handle, handle);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_ubo_binding() {
        let mut ctx = setup_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024).unwrap();
        let buffer = ctx
            .make_buffer(&BufferInfo {
                debug_name: "ubo_test",
                byte_size: 16,
                visibility: MemoryVisibility::CpuAndGpu,
                usage: DashiBufferUsage::UNIFORM,
                initial_data: None,
            })
            .unwrap();

        manager.register_ubo("ubo", buffer);
        match manager.get("ubo") {
            Some(ResourceBinding::Uniform(h)) => assert_eq!(*h, buffer),
            _ => panic!("Expected uniform binding"),
        }
        assert!(manager.buffers.entries.is_empty());
        ctx.destroy_buffer(buffer);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_storage_binding() {
        let mut ctx = setup_ctx();
        let mut manager = ResourceManager::new(&mut ctx, 1024).unwrap();
        let buffer = ctx
            .make_buffer(&BufferInfo {
                debug_name: "storage_test",
                byte_size: 16,
                visibility: MemoryVisibility::CpuAndGpu,
                usage: DashiBufferUsage::STORAGE,
                initial_data: None,
            })
            .unwrap();

        manager.register_storage("store", buffer);
        match manager.get("store") {
            Some(ResourceBinding::Storage(h)) => assert_eq!(*h, buffer),
            _ => panic!("Expected storage binding"),
        }
        assert!(manager.buffers.entries.is_empty());
        ctx.destroy_buffer(buffer);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn register_combined_texture_array_binding() {
        let mut ctx = setup_ctx();
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
        let mut list = ResourceList::<CombinedTextureSampler>::default();

        let img = ctx.make_image(&ImageInfo::default()).unwrap();
        let view = ctx
            .make_image_view(&ImageViewInfo { img, ..Default::default() })
            .unwrap();
        list.push(CombinedTextureSampler {
            texture: Texture {
                handle: img,
                view,
                dim: [1, 1],
            },
            sampler,
        });

        let array = Arc::new(list);
        let mut manager = ResourceManager::new(&mut ctx, 1024).unwrap();
        manager.register_combined_texture_array("combo_arr", array.clone());

        match manager.get("combo_arr") {
            Some(ResourceBinding::CombinedTextureArray(arr)) => {
                assert!(Arc::ptr_eq(arr, &array));
            }
            _ => panic!("Expected combined texture array"),
        }
        ctx.destroy();
    }

    #[test]
    fn invalid_lookup_returns_none() {
        let manager = ResourceManager::default();
        assert!(manager.get("missing").is_none());
    }
}
