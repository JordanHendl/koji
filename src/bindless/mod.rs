use dashi::{
    utils::Handle, BindGroup, BindGroupInfo, BindGroupLayout, BindGroupLayoutInfo,
    BindGroupUpdateInfo, BindGroupVariable, BindGroupVariableType, Buffer, BufferInfo, BufferUsage,
    Context, Image, ImageView, IndexedBindGroupInfo, IndexedBindingInfo, IndexedResource,
    MemoryVisibility, Sampler, ShaderInfo, ShaderResource, ShaderType,
};
use glam::Vec4;

use crate::utils::ResourceList;

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

#[derive(Default, Clone, Copy)]
#[repr(packed)]
pub struct MaterialInfo {
    pub base_color_factor: Vec4,
    pub emissive_factor: Vec4,
    pub base_color: Handle<Texture>,
    pub normal: Handle<Texture>,
    pub emissive: Handle<Texture>,
}

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct Material {
    pub(crate) buff: Handle<Buffer>,
}

#[derive(Default)]
struct ResourceManager {
    pub materials: ResourceList<Material>,
    pub textures: ResourceList<Texture>,
    //    pub lights: LightCollection,
}

#[allow(dead_code)]
pub struct BindlessManager {
    ctx: *mut Context,
    res: ResourceManager,
    bg: Handle<BindGroup>,
    layout: Handle<BindGroupLayout>,
}

static BINDLESS_SET: u32 = 0;
static BINDLESS_TEXTURE_BINDING: u32 = 10;
static BINDLESS_MATERIAL_BINDING: u32 = 11;
static BINDLESS_DYNAMIC_BINDING: u32 = 20;
impl BindlessManager {
    pub fn new(ctx: &mut Context) -> Self {
        let layout = ctx
            .make_bind_group_layout(&BindGroupLayoutInfo {
                debug_name: "[KOJI] Bindless Bind Group Layout",
                shaders: &[ShaderInfo {
                    shader_type: ShaderType::All,
                    variables: &[
                        BindGroupVariable {
                            var_type: BindGroupVariableType::SampledImage,
                            binding: BINDLESS_TEXTURE_BINDING,
                        },
                        BindGroupVariable {
                            var_type: BindGroupVariableType::Uniform,
                            binding: BINDLESS_MATERIAL_BINDING,
                        },
                        BindGroupVariable {
                            var_type: BindGroupVariableType::DynamicStorage,
                            binding: BINDLESS_DYNAMIC_BINDING,
                        },
                    ],
                }],
            })
            .unwrap();

        let bg = ctx
            .make_indexed_bind_group(&IndexedBindGroupInfo {
                debug_name: "[KOJI] Bindless Bind Group",
                layout,
                bindings: &[],
                set: BINDLESS_SET,
            })
            .unwrap();

        Self {
            ctx,
            res: Default::default(),
            bg,
            layout,
        }
    }

    pub fn register_texture(&mut self, info: &TextureInfo) -> Handle<Texture> {
        let h = self.res.textures.push(Texture {
            handle: info.image,
            sampler: info.sampler,
            dim: info.dim,
            view: info.view,
        });

        let bg = self.bg;
        self.get_ctx().update_bind_group(&BindGroupUpdateInfo {
            bg,
            bindings: &[IndexedBindingInfo {
                resources: &[IndexedResource {
                    resource: ShaderResource::SampledImage(info.view, info.sampler),
                    slot: h.slot as u32,
                }],
                binding: BINDLESS_TEXTURE_BINDING,
            }],
        });

        h
    }

    pub fn register_material(&mut self, name: &str, info: &MaterialInfo) -> Handle<Material> {
        let buff = self
            .get_ctx()
            .make_buffer(&BufferInfo {
                debug_name: name,
                byte_size: std::mem::size_of::<MaterialInfo>() as u32,
                visibility: MemoryVisibility::Gpu,
                usage: BufferUsage::ALL,
                initial_data: Some(unsafe { &[info].align_to::<u8>().1 }),
            })
            .unwrap();

        let h = self.res.materials.push(Material { buff });
        let bg = self.bg;

        self.get_ctx().update_bind_group(&BindGroupUpdateInfo {
            bg,
            bindings: &[IndexedBindingInfo {
                resources: &[IndexedResource {
                    resource: ShaderResource::Buffer(buff),
                    slot: h.slot as u32,
                }],
                binding: BINDLESS_MATERIAL_BINDING,
            }],
        });

        h
    }

    pub fn shutdown(&mut self) {}

    fn get_ctx(&mut self) -> &mut Context {
        unsafe { &mut (*self.ctx) }
    }
}
