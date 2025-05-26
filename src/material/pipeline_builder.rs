use crate::material::*;
use crate::utils::{DHObject, ResourceBinding, ResourceBuffer, Texture};
use bytemuck::Pod;
use dashi::*;
use std::collections::HashMap;

use spirv_reflect::types::ReflectFormat;
use spirv_reflect::ShaderModule;

use self::shader_reflection::*;

/// Map shader descriptor types to Dashi bind group variable types
fn descriptor_type_to_dashi(ty: ShaderDescriptorType) -> BindGroupVariableType {
    match ty {
        ShaderDescriptorType::SampledImage | ShaderDescriptorType::CombinedImageSampler => {
            BindGroupVariableType::SampledImage
        }
        ShaderDescriptorType::UniformBuffer => BindGroupVariableType::Uniform,
        ShaderDescriptorType::StorageBuffer => BindGroupVariableType::Storage,
        ShaderDescriptorType::StorageImage => BindGroupVariableType::StorageImage,
        other => panic!("Unsupported descriptor type: {:?}", other),
    }
}

/// Map SPIR-V reflect format to shader primitive enum
fn reflect_format_to_shader_primitive(fmt: ReflectFormat) -> ShaderPrimitiveType {
    use ReflectFormat::*;
    match fmt {
        R32G32B32A32_SFLOAT => ShaderPrimitiveType::Vec4,
        R32G32B32_SFLOAT => ShaderPrimitiveType::Vec3,
        R32G32_SFLOAT => ShaderPrimitiveType::Vec2,
        other => panic!("Unsupported vertex input format: {:?}", other),
    }
}

pub struct ShaderVariable {
    allocation: crate::utils::DHObject,
    members: Vec<(String, u32, u32)>,
    ctx: *mut Context,
    set: usize,
    binding: u32,
}

impl ShaderVariable {
    // Writes to a specific member of this object.
    pub fn write_member<T: Pod>(&self, field: &str, value: T) {
        let ctx = unsafe { &mut *self.ctx };
        let (_, offset, size) = self
            .members
            .iter()
            .find(|(name, _, _)| name == field)
            .expect("Field not found");
        assert!(std::mem::size_of::<T>() <= *size as usize, "Size mismatch");

        let slice = unsafe { ctx.map_buffer_mut(self.allocation.handle).unwrap() };
        let bytes = bytemuck::bytes_of(&value);
        slice[(self.allocation.offset + *offset as u64) as usize..][..bytes.len()]
            .copy_from_slice(bytes);

        ctx.unmap_buffer(self.allocation.handle).unwrap();
    }

    // Writes to the whole field. size of<T> must equal the size of the underlying shader variable.
    pub fn write<T: Pod>(&self, value: T) {
        let ctx = unsafe { &mut *self.ctx };
        assert!(
            std::mem::size_of::<T>() <= self.allocation.size as usize,
            "Size mismatch"
        );

        let slice = unsafe { ctx.map_buffer_mut(self.allocation.handle).unwrap() };
        let bytes = bytemuck::bytes_of(&value);
        slice[self.allocation.offset as usize..][..bytes.len()].copy_from_slice(bytes);

        ctx.unmap_buffer(self.allocation.handle).unwrap();
    }

    pub fn read_member<T: Pod>(&self, field: &str) -> T {
        let ctx = unsafe { &mut *self.ctx };
        let (_, offset, size) = self
            .members
            .iter()
            .find(|(name, _, _)| name == field)
            .expect("Field not found");
        assert!(std::mem::size_of::<T>() <= *size as usize, "Size mismatch");

        let slice = unsafe { ctx.map_buffer::<u8>(self.allocation.handle).unwrap() };
        let data_slice = &slice[(self.allocation.offset + *offset as u64) as usize..];
        let value = bytemuck::from_bytes::<T>(&data_slice[..std::mem::size_of::<T>()]);

        let cln = unsafe { std::mem::transmute_copy(value) };
        ctx.unmap_buffer(self.allocation.handle).unwrap();
        cln
    }

    pub fn read<T: Pod>(&self) -> T {
        let ctx = unsafe { &mut *self.ctx };
        assert!(
            std::mem::size_of::<T>() <= self.allocation.size as usize,
            "Size mismatch"
        );

        let slice = unsafe { ctx.map_buffer::<u8>(self.allocation.handle).unwrap() };
        let data_slice = &slice[self.allocation.offset as usize..];
        let value = bytemuck::from_bytes::<T>(&data_slice[..std::mem::size_of::<T>()]);

        let cln = unsafe { std::mem::transmute_copy(value) };
        ctx.unmap_buffer(self.allocation.handle).unwrap();
        cln
    }
}

pub struct PSOResource {
    binding: u32,
    variables: Vec<(String, ShaderVariable)>,
}

impl PSOResource {
    pub fn binding(&self) -> u32 {
        self.binding
    }

    pub fn variable(&mut self, name: &str) -> Option<&ShaderVariable> {
        self.variables
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, var)| var)
    }
}

pub struct PSOBindGroupResources {
    pub bind_group: Handle<BindGroup>,
    pub buffers: HashMap<String, Handle<Buffer>>,
    pub textures: HashMap<String, Texture>,
}

/// Builder for a graphics pipeline, including reflection of SPIR-V
pub struct PipelineBuilder<'a> {
    ctx: &'a mut Context,
    vert_spirv: &'a [u32],
    frag_spirv: &'a [u32],
    render_pass: Option<Handle<RenderPass>>,
    pipeline_name: &'static str,
    depth_enable: bool,
    cull_mode: CullMode,
    subpass: u32,
}

/// A pipeline state object (PSO) that holds the GPU pipeline handle,
/// its associated layout, bind group layouts, and reflection info for creating bind groups by name.
pub struct PSO {
    pub pipeline: Handle<GraphicsPipeline>,
    pub layout: Handle<GraphicsPipelineLayout>,
    pub bind_group_layouts: [Option<Handle<BindGroupLayout>>; 4],
    /// Mapping from descriptor name to (set_index, binding_index, block_size)
    desc_map: HashMap<String, (usize, u32, u32)>,
    ctx: *mut Context,
}

impl PSO {
    /// Create a bind group for the given set index with provided bindings.
    pub fn create_bind_group(
        &mut self,
        set_index: usize,
        resources: &ResourceManager,
    ) -> PSOBindGroupResources {
        let ctx = unsafe { &mut *self.ctx };
        let layout = self.bind_group_layouts[set_index].expect("Bind group layout not initialized");

        let mut bindings = Vec::new();
        let mut buffers = HashMap::new();
        let mut textures = HashMap::new();

        // This holds the real data for all indexed arrays!
        let mut all_indexed_data: Vec<Vec<IndexedResource>> = Vec::new();
        let mut which_binding: Vec<(usize, usize)> = Vec::new(); // (vec_idx, binding)
        for (name, (set, binding, count)) in self.desc_map.iter() {
            if *set != set_index {
                continue;
            }
            if let Some(binding_entry) = resources.get(name) {
                match binding_entry {
                    ResourceBinding::Uniform(b) => {
                        buffers.insert(name.clone(), b.clone());
                        bindings.push(BindingInfo {
                            resource: ShaderResource::Buffer(*b),
                            binding: *binding,
                        });
                    }
                    ResourceBinding::Storage(b) => {
                        buffers.insert(name.clone(), b.clone());
                        bindings.push(BindingInfo {
                            resource: ShaderResource::StorageBuffer(*b),
                            binding: *binding,
                        });
                    }
                    ResourceBinding::Texture(t) => {
                        textures.insert(name.clone(), t.clone());
                        bindings.push(BindingInfo {
                            resource: ShaderResource::SampledImage(t.view, Handle::default()),
                            binding: *binding,
                        });
                    }
                    ResourceBinding::CombinedImageSampler { texture, sampler } => {
                        textures.insert(name.clone(), texture.clone());
                        bindings.push(BindingInfo {
                            resource: ShaderResource::SampledImage(texture.view, *sampler),
                            binding: *binding,
                        });
                    }
                    ResourceBinding::TextureArray(array) => {
                        let mut data: Vec<IndexedResource> = array
                            .as_ref()
                            .iter()
                            .enumerate()
                            .map(|(i, t)| IndexedResource {
                                resource: ShaderResource::SampledImage(t.view, Handle::default()),
                                slot: i as u32,
                            })
                            .collect();
                        if *count > 1 {
                            data.truncate(*count as usize);
                        }
                        all_indexed_data.push(data);
                        which_binding.push((all_indexed_data.len() - 1, *binding as usize));
                    }
                    ResourceBinding::CombinedTextureArray(array) => {
                        let mut data: Vec<IndexedResource> = array
                            .as_ref()
                            .iter()
                            .enumerate()
                            .map(|(i, ts)| IndexedResource {
                                resource: ShaderResource::SampledImage(ts.texture.view, ts.sampler),
                                slot: i as u32,
                            })
                            .collect();
                        if *count > 1 {
                            data.truncate(*count as usize);
                        }

                        all_indexed_data.push(data);
                        which_binding.push((all_indexed_data.len() - 1, *binding as usize));
                    }
                    ResourceBinding::BufferArray(array) => {
                        let mut data: Vec<IndexedResource> = array
                            .as_ref()
                            .iter()
                            .enumerate()
                            .map(|(i, b)| IndexedResource {
                                resource: ShaderResource::StorageBuffer(b.handle),
                                slot: i as u32,
                            })
                            .collect();
                        if *count > 1 {
                            data.truncate(*count as usize);
                        }

                        all_indexed_data.push(data);

                        which_binding.push((all_indexed_data.len() - 1, *binding as usize));
                    }
                }
            } else {
                panic!("Resource not found: {}", name);
            }
        }
        // Now build all references in a *second pass*
        let indexed_bindings: Vec<IndexedBindingInfo> = which_binding
            .iter()
            .map(|(vec_idx, binding)| IndexedBindingInfo {
                resources: &all_indexed_data[*vec_idx],
                binding: *binding as u32,
            })
            .collect();
        let bind_group = if !indexed_bindings.is_empty() {
            ctx.make_indexed_bind_group(&IndexedBindGroupInfo {
                debug_name: "Bindless PSO bind group",
                layout,
                bindings: &indexed_bindings,
                set: set_index as u32,
                ..Default::default()
            })
            .unwrap()
        } else {
            ctx.make_bind_group(&BindGroupInfo {
                debug_name: "Auto-generated PSO bind group",
                layout,
                set: set_index as u32,
                bindings: &bindings,
                ..Default::default()
            })
            .unwrap()
        };

        PSOBindGroupResources {
            bind_group,
            buffers,
            textures,
        }
    }
}

impl<'a> PipelineBuilder<'a> {
    /// Create a new builder with context and pipeline name
    pub fn new(ctx: &'a mut Context, name: &'static str) -> Self {
        Self {
            ctx,
            pipeline_name: name,
            vert_spirv: &[],
            frag_spirv: &[],
            render_pass: None,
            subpass: 0,
            depth_enable: false,
            cull_mode: CullMode::None,
        }
    }

    pub fn depth_enable(mut self, enable: bool) -> Self {
        self.depth_enable = enable;
        self
    }

    pub fn cull_mode(mut self, mode: CullMode) -> Self {
        self.cull_mode = mode;
        self
    }
    /// Set the vertex SPIR-V bytecode
    pub fn vertex_shader(mut self, spirv: &'a [u32]) -> Self {
        self.vert_spirv = spirv;
        self
    }

    /// Set the fragment SPIR-V bytecode
    pub fn fragment_shader(mut self, spirv: &'a [u32]) -> Self {
        self.frag_spirv = spirv;
        self
    }

    /// Specify the render pass and its subpass index
    pub fn render_pass(mut self, rp: Handle<RenderPass>, subpass: u32) -> Self {
        self.render_pass = Some(rp);
        self.subpass = subpass;
        self
    }

    /// Build and return the graphics pipeline handle
    pub fn build(self) -> PSO {
        let rp = self
            .render_pass
            .expect("Render pass must be set before build");

        let vert_info = reflect_shader(self.vert_spirv);
        let frag_info = reflect_shader(self.frag_spirv);

        let mut combined: HashMap<u32, Vec<ShaderDescriptorBinding>> = HashMap::new();
        for (set, binds) in vert_info.bindings.into_iter().chain(frag_info.bindings) {
            combined.entry(set).or_default().extend(binds);
        }

        let mut desc_map = HashMap::new();
        let mut bg_layouts: [Option<Handle<BindGroupLayout>>; 4] = [None, None, None, None];

        for set in combined.keys().cloned().collect::<Vec<_>>() {
            let binds = &combined[&set];
            let mut vars = Vec::new();

            for b in binds.iter() {
                let var_type = descriptor_type_to_dashi(b.ty);
                vars.push(BindGroupVariable {
                    var_type,
                    binding: b.binding,
                    count: b.count,
                });
                desc_map.insert(b.name.clone(), (set as usize, b.binding, b.count));
            }

            let info = BindGroupLayoutInfo {
                debug_name: self.pipeline_name,
                shaders: &[ShaderInfo {
                    shader_type: ShaderType::All,
                    variables: &vars,
                }],
            };
            let layout = self.ctx.make_bind_group_layout(&info).unwrap();
            bg_layouts[set as usize] = Some(layout);
        }

        let module = ShaderModule::load_u32_data(self.vert_spirv).unwrap();
        let mut inputs = module.enumerate_input_variables(None).unwrap();
        inputs.sort_by_key(|v| v.location);

        let mut entries = Vec::new();
        let mut offset = 0;
        for var in inputs {
            let fmt = reflect_format_to_shader_primitive(var.format);
            entries.push(VertexEntryInfo {
                format: fmt,
                location: var.location as usize,
                offset: offset as usize,
            });
            offset += match fmt {
                ShaderPrimitiveType::Vec4 | ShaderPrimitiveType::IVec4 => 16,
                ShaderPrimitiveType::Vec3 => 12,
                ShaderPrimitiveType::Vec2 => 8,
                _ => 0,
            };
        }

        let vertex_info = VertexDescriptionInfo {
            entries: &entries,
            stride: offset as usize,
            rate: VertexRate::Vertex,
        };

        let layout_info = GraphicsPipelineLayoutInfo {
            debug_name: self.pipeline_name,
            vertex_info,
            bg_layouts,
            shaders: &[
                PipelineShaderInfo {
                    stage: ShaderType::Vertex,
                    spirv: self.vert_spirv,
                    specialization: &[],
                },
                PipelineShaderInfo {
                    stage: ShaderType::Fragment,
                    spirv: self.frag_spirv,
                    specialization: &[],
                },
            ],
            details: GraphicsPipelineDetails {
                subpass: self.subpass as u8,
                color_blend_states: vec![ColorBlendState::default()],
                topology: Topology::TriangleList,
                culling: self.cull_mode,
                front_face: VertexOrdering::CounterClockwise,
                depth_test: if self.depth_enable {
                    Some(DepthInfo {
                        should_test: true,
                        should_write: true,
                    })
                } else {
                    None
                },
                ..Default::default()
            },
        };

        let layout = self
            .ctx
            .make_graphics_pipeline_layout(&layout_info)
            .unwrap();

        let pipeline_handle = self
            .ctx
            .make_graphics_pipeline(&GraphicsPipelineInfo {
                debug_name: self.pipeline_name,
                layout,
                render_pass: rp,
                subpass_id: self.subpass as u8,
                ..Default::default()
            })
            .unwrap();

        PSO {
            pipeline: pipeline_handle,
            layout,
            bind_group_layouts: bg_layouts,
            desc_map,
            ctx: self.ctx,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        material::pipeline_builder::ShaderDescriptorType,
        utils::{
            allocator::GpuAllocator, resource_list::ResourceList, CombinedTextureSampler,
            TextureInfo,
        },
    };
    use dashi::builders::RenderPassBuilder;
    use inline_spirv::inline_spirv;
    use serial_test::serial;
    use spirv_reflect::types::ReflectFormat;

    fn make_ctx() -> Context {
        Context::headless(&ContextInfo::default()).unwrap()
    }
    fn simple_vert() -> Vec<u32> {
        inline_spirv!(
            r#"
            #version 450
            layout(set=0,binding=0) uniform U{vec4 u;};
            layout(location=0) in vec2 v;
            void main(){ gl_Position=vec4(v,0,1); }"#,
            vert
        )
        .to_vec()
    }
    fn simple_frag() -> Vec<u32> {
        inline_spirv!(
            r#"
            #version 450
            layout(set=0,binding=1) uniform U2{float x;};
            layout(location=0) out vec4 o;
            void main(){ o=vec4(x); }"#,
            frag
        )
        .to_vec()
    }

    fn simple_vertex_spirv() -> Vec<u32> {
        inline_spirv!(
            r#"
            #version 450
            layout(location=0) in vec2 pos;
            void main(){ gl_Position=vec4(pos,0,1);}"#,
            vert
        )
        .to_vec()
    }
    fn simple_fragment_spirv() -> Vec<u32> {
        inline_spirv!(
            r#"
            #version 450
            layout(location=0) out vec4 outCol;
            void main(){ outCol=vec4(1); }"#,
            frag
        )
        .to_vec()
    }

    #[test]
    #[serial]
    fn builder_with_no_descriptors_creates_pipeline() {
        let mut ctx = make_ctx();
        // make minimal render pass
        let viewport = Viewport::default();
        let rp = RenderPassBuilder::new("rp", viewport)
            .add_subpass(&[AttachmentDescription::default()], None, &[])
            .build(&mut ctx)
            .unwrap();

        let vert = simple_vertex_spirv();
        let frag = simple_fragment_spirv();

        let pipeline = PipelineBuilder::new(&mut ctx, "test_no_desc")
            .vertex_shader(&vert)
            .fragment_shader(&frag)
            .render_pass(rp, 0)
            .build();

        assert!(pipeline.pipeline.valid());
        //        ctx.destroy_graphics_pipeline(pipeline);
        ctx.destroy();
    }

    #[test]
    #[serial]
    #[should_panic(expected = "Render pass must be set before build")]
    fn pipeline_panics_without_render_pass() {
        let mut ctx = make_ctx();
        let vert = simple_vertex_spirv();
        let frag = simple_fragment_spirv();

        // Missing render pass => should panic
        PipelineBuilder::new(&mut ctx, "bad")
            .vertex_shader(&vert)
            .fragment_shader(&frag)
            .build();
    }

    #[test]
    #[serial]
    fn descriptor_mapping_roundtrip() {
        assert_eq!(
            descriptor_type_to_dashi(ShaderDescriptorType::SampledImage),
            BindGroupVariableType::SampledImage
        );
        assert_eq!(
            descriptor_type_to_dashi(ShaderDescriptorType::UniformBuffer),
            BindGroupVariableType::Uniform
        );
    }

    #[test]
    #[serial]
    fn reflect_format_mapping() {
        use ReflectFormat::*;
        assert_eq!(
            reflect_format_to_shader_primitive(R32G32_SFLOAT),
            ShaderPrimitiveType::Vec2
        );
    }

    #[test]
    #[serial]
    #[should_panic]
    fn out_of_range_descriptor_set_panics() {
        let mut ctx = make_ctx();
        let viewport = Viewport::default();
        let rp = RenderPassBuilder::new("rp", viewport)
            .add_subpass(&[AttachmentDescription::default()], None, &[])
            .build(&mut ctx)
            .unwrap();
        let vert = inline_spirv!(
            r#"
            #version 450
            layout(set=5,binding=0) uniform U{float x;};
            void main(){}
        "#,
            vert
        )
        .to_vec();
        let frag = simple_fragment_spirv();

        // should panic on build
        let _ = PipelineBuilder::new(&mut ctx, "oops")
            .vertex_shader(&vert)
            .fragment_shader(&frag)
            .render_pass(rp, 0)
            .build();
        ctx.destroy();
    }

    fn setup_ctx() -> gpu::Context {
        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        gpu::Context::new(&ContextInfo { device }).unwrap()
    }

    #[test]
    #[serial]
    fn shader_variable_write() {
        let mut ctx = setup_ctx();
        let buffer_handle = ctx
            .make_buffer(&BufferInfo {
                debug_name: "test_buffer",
                byte_size: 4,
                visibility: MemoryVisibility::CpuAndGpu,
                usage: BufferUsage::STORAGE,
                initial_data: None,
            })
            .unwrap();

        let allocation = DHObject {
            handle: buffer_handle,
            offset: 0,
            size: 4,
        };

        let variable = ShaderVariable {
            allocation,
            members: vec![("data".into(), 0, 4)],
            ctx: &mut ctx,
            set: 0,
            binding: 0,
        };

        variable.write(100u32);
        let read_back: u32 = variable.read();
        assert_eq!(read_back, 100);

        variable.write_member("data", 200u32);
        let read_member_back: u32 = variable.read_member("data");
        assert_eq!(read_member_back, 200);

        ctx.destroy_buffer(buffer_handle);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn pso_resource_variable_lookup() {
        let variable = ShaderVariable {
            allocation: DHObject {
                handle: Handle::default(),
                offset: 0,
                size: 4,
            },
            members: vec![],
            ctx: std::ptr::null_mut(),
            set: 0,
            binding: 0,
        };

        let mut resource = PSOResource {
            binding: 0,
            variables: vec![("var1".into(), variable)],
        };

        assert!(resource.variable("var1").is_some());
        assert!(resource.variable("nonexistent").is_none());
    }

    fn simple_vert2() -> Vec<u32> {
        inline_spirv!(
            r#"
            #version 450
            layout(location=0) in vec2 pos;
            layout(set=0, binding=0) uniform B0 { uint x; } b0;
            void main() {
                gl_Position = vec4(pos, 0.0, 1.0);
            }
            "#,
            vert
        )
        .to_vec()
    }

    fn simple_frag2() -> Vec<u32> {
        inline_spirv!(
            r#"
            #version 450
            layout(set=0, binding=1) uniform sampler2D tex;
            layout(location=0) out vec4 o;
            void main() {
                o = texture(tex, vec2(0.5));
            }
            "#,
            frag
        )
        .to_vec()
    }

    #[test]
    #[serial]
    fn pipeline_builder_and_bind_group() {
        let mut ctx = make_ctx();

        let rp = RenderPassBuilder::new("rp", Viewport::default())
            .add_subpass(&[AttachmentDescription::default()], None, &[])
            .build(&mut ctx)
            .unwrap();

        let mut pso = PipelineBuilder::new(&mut ctx, "pso_test")
            .vertex_shader(&simple_vert2())
            .fragment_shader(&simple_frag2())
            .render_pass(rp, 0)
            .build();

        let mut resources = ResourceManager::new(&mut ctx, 1024).unwrap();

        resources.register_variable("b0", &mut ctx, 1234u32);

        let img = ctx.make_image(&ImageInfo::default()).unwrap();
        let view = ctx
            .make_image_view(&ImageViewInfo {
                img,
                ..Default::default()
            })
            .unwrap();
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();

        resources.register_combined("tex", img, view, [1, 1], sampler);

        let group = pso.create_bind_group(0, &resources);

        assert!(group.bind_group.valid());
        assert!(group.buffers.contains_key("b0"));
        assert!(group.textures.contains_key("tex"));

        ctx.destroy();
    }

    #[test]
    #[serial]
    fn bindless_texture_array_in_shader() {
        let mut ctx = make_ctx();
        let rp = RenderPassBuilder::new("rp", Viewport::default())
            .add_subpass(&[AttachmentDescription::default()], None, &[])
            .build(&mut ctx)
            .unwrap();

        // Vertex shader (minimal, no array)
        let vert_spirv = inline_spirv!(
            r#"
            #version 450
            layout(location=0) in vec2 pos;
            void main() {
                gl_Position = vec4(pos, 0.0, 1.0);
            }
            "#,
            vert
        )
        .to_vec();

        // Fragment shader with bindless texture array
        let frag_spirv = inline_spirv!(
            r#"
            #version 450
            layout(set=0, binding=0) uniform sampler2D bless_textures[];
            layout(location=0) out vec4 o;
            void main() {
                // Index 2 for test, would be dynamic in real shaders
                o = texture(bless_textures[2], vec2(0.5));
            }
            "#,
            frag
        )
        .to_vec();

        let mut pso = PipelineBuilder::new(&mut ctx, "pso_bindless_test")
            .vertex_shader(&vert_spirv)
            .fragment_shader(&frag_spirv)
            .render_pass(rp, 0)
            .build();

        let sampler = ctx.make_sampler(&Default::default()).unwrap();
        // Register a texture array (bindless)
        let mut tex_array = ResourceList::<CombinedTextureSampler>::default();
        for _ in 0..4 {
            let img = ctx.make_image(&ImageInfo::default()).unwrap();
            let view = ctx
                .make_image_view(&ImageViewInfo {
                    img,
                    ..Default::default()
                })
                .unwrap();
            tex_array.push(CombinedTextureSampler {
                texture: Texture {
                    handle: img,
                    view,
                    dim: [32, 32],
                },
                sampler,
            });
        }
        let tex_array = Arc::new(tex_array);
        let mut resources = ResourceManager::new(&mut ctx, 1024).unwrap();
        resources.register_combined_texture_array("bless_textures", tex_array.clone());

        // The pipeline should reflect the unsized array and request the bindless resource
        let group = pso.create_bind_group(0, &resources);

        // Expect a valid bind group, and that "bless_textures" is registered as a texture array
        assert!(group.bind_group.valid());
        assert!(resources.get("bless_textures").is_some());

        ctx.destroy();
    }

    #[test]
    #[serial]
    fn multiple_bindless_bindings_in_shader() {
        let mut ctx = make_ctx();
        let rp = RenderPassBuilder::new("rp", Viewport::default())
            .add_subpass(&[AttachmentDescription::default()], None, &[])
            .build(&mut ctx)
            .unwrap();

        // Vertex shader: pass through
        let vert_spirv = inline_spirv!(
            r#"
            #version 450
            layout(location=0) in vec2 pos;
            void main() { gl_Position = vec4(pos, 0.0, 1.0); }
            "#,
            vert
        )
        .to_vec();

        // Fragment shader: bindless combined sampler2D array at binding=0, buffer array at binding=1
        let frag_spirv = inline_spirv!(
            r#"
            #version 450
            layout(set=0, binding=0) uniform sampler2D tex_array[];
            layout(set=0, binding=1) buffer Bufs { uint val[]; } buf_array[];
            layout(location=0) out vec4 o;
            void main() {
                // Sample from tex_array[2] and read from buf_array[2].val[0]
                vec4 c = texture(tex_array[2], vec2(0.5));
                float v = buf_array[2].val[0];
                o = c + vec4(v);
            }
            "#,
            frag
        )
        .to_vec();

        let mut pso = PipelineBuilder::new(&mut ctx, "bindless_combined_and_buffer_array_test")
            .vertex_shader(&vert_spirv)
            .fragment_shader(&frag_spirv)
            .render_pass(rp, 0)
            .build();

        let mut combined_array = ResourceList::<CombinedTextureSampler>::default();
        let mut buf_array = ResourceList::<ResourceBuffer>::default();

        for _ in 0..4 {
            let img = ctx.make_image(&ImageInfo::default()).unwrap();
            let view = ctx
                .make_image_view(&ImageViewInfo {
                    img,
                    ..Default::default()
                })
                .unwrap();
            let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
            let c = CombinedTextureSampler {
                texture: Texture {
                    handle: img,
                    view,
                    dim: [32, 32],
                },
                sampler,
            };
            combined_array.push(c);

            let mut allocator =
                GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 256).unwrap();
            let dh = DHObject::new(&mut ctx, &mut allocator, 123u32).unwrap();
            buf_array.push(ResourceBuffer::from(dh));
        }

        let mut resources = ResourceManager::new(&mut ctx, 4096).unwrap();
        resources.register_combined_texture_array("tex_array", Arc::new(combined_array));
        resources.register_buffer_array("buf_array", Arc::new(buf_array));

        let group = pso.create_bind_group(0, &resources);

        assert!(group.bind_group.valid());
        assert!(matches!(
            resources.get("tex_array"),
            Some(ResourceBinding::CombinedTextureArray(_))
        ));
        assert!(matches!(
            resources.get("buf_array"),
            Some(ResourceBinding::BufferArray(_))
        ));

        ctx.destroy();
    }
}
