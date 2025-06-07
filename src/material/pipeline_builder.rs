use crate::material::*;
use crate::utils::{ResourceBinding, Texture};
use bytemuck::Pod;
use std::collections::HashMap;

use spirv_reflect::types::ReflectFormat;
use spirv_reflect::ShaderModule;


/// Map SPIR-V reflect format to shader primitive enum
pub(crate) fn reflect_format_to_shader_primitive(fmt: ReflectFormat) -> ShaderPrimitiveType {
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

        let slice = ctx.map_buffer_mut(self.allocation.handle).unwrap();
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

        let slice = ctx.map_buffer_mut(self.allocation.handle).unwrap();
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

        let slice = ctx.map_buffer::<u8>(self.allocation.handle).unwrap();
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

        let slice = ctx.map_buffer::<u8>(self.allocation.handle).unwrap();
        let data_slice = &slice[self.allocation.offset as usize..];
        let value = bytemuck::from_bytes::<T>(&data_slice[..std::mem::size_of::<T>()]);

        let cln = unsafe { std::mem::transmute_copy(value) };
        ctx.unmap_buffer(self.allocation.handle).unwrap();
        cln
    }
}

#[cfg(test)]
impl ShaderVariable {
    pub fn test_new(
        allocation: crate::utils::DHObject,
        members: Vec<(String, u32, u32)>,
        ctx: *mut Context,
    ) -> Self {
        Self {
            allocation,
            members,
            ctx,
        }
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

#[cfg(test)]
impl PSOResource {
    pub fn test_new(binding: u32, variables: Vec<(String, ShaderVariable)>) -> Self {
        Self { binding, variables }
    }
}

pub struct PSOBindGroupResources {
    pub bind_group: Handle<BindGroup>,
    pub buffers: HashMap<String, Handle<Buffer>>,
    pub textures: HashMap<String, Texture>,
}

#[derive(Debug)]
pub enum PipelineError {
    MissingResource(String),
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
    ) -> Result<PSOBindGroupResources, PipelineError> {
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
                        let list = array.lock().unwrap();
                        let mut data: Vec<IndexedResource> = list
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
                return Err(PipelineError::MissingResource(name.clone()));
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

        Ok(PSOBindGroupResources {
            bind_group,
            buffers,
            textures,
        })
    }

    pub fn create_bind_groups(
        &mut self,
        res: &ResourceManager,
    ) -> Result<[Option<PSOBindGroupResources>; 4], PipelineError> {
        let mut sets: [Option<PSOBindGroupResources>; 4] = [None, None, None, None];
        for set_idx in 0..4 {
            if self.bind_group_layouts[set_idx].is_some() {
                sets[set_idx] = Some(self.create_bind_group(set_idx, res)?);
            }
        }
        Ok(sets)
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
                let var_type = descriptor_to_var_type(b.ty);
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

