use crate::material::*;
use crate::utils::{ResourceBinding, ResourceManager};
use std::collections::HashMap;

/// Internal defaults for auto-registered resources
enum DefaultResource {
    Time,
}

const DEFAULT_RESOURCES: &[(&str, DefaultResource)] = &[("KOJI_time", DefaultResource::Time)];

pub struct CPSO {
    pub pipeline: Handle<ComputePipeline>,
    pub layout: Handle<ComputePipelineLayout>,
    pub bind_group_layouts: [Option<Handle<BindGroupLayout>>; 4],
    desc_map: HashMap<String, (usize, u32, u32)>,
    ctx: *mut Context,
}

impl CPSO {
    pub fn create_bind_group(
        &mut self,
        set_index: usize,
        resources: &ResourceManager,
    ) -> Result<PSOBindGroupResources, PipelineError> {
        let ctx = unsafe { &mut *self.ctx };
        let layout = self.bind_group_layouts[set_index]
            .expect("Bind group layout not initialized");

        let mut bindings = Vec::new();
        let mut buffers = HashMap::new();
        let mut textures = HashMap::new();

        let mut all_indexed_data: Vec<Vec<IndexedResource>> = Vec::new();
        let mut which_binding: Vec<(usize, usize)> = Vec::new();
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

        let indexed_bindings: Vec<IndexedBindingInfo> = which_binding
            .iter()
            .map(|(vec_idx, binding)| IndexedBindingInfo {
                resources: &all_indexed_data[*vec_idx],
                binding: *binding as u32,
            })
            .collect();
        let bind_group = if !indexed_bindings.is_empty() {
            ctx.make_indexed_bind_group(&IndexedBindGroupInfo {
                debug_name: "Bindless CPSO bind group",
                layout,
                bindings: &indexed_bindings,
                set: set_index as u32,
                ..Default::default()
            })
            .unwrap()
        } else {
            ctx.make_bind_group(&BindGroupInfo {
                debug_name: "Auto-generated CPSO bind group",
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

pub struct ComputePipelineBuilder<'a> {
    ctx: &'a mut Context,
    shader_spirv: &'a [u32],
    pipeline_name: &'static str,
}

impl<'a> ComputePipelineBuilder<'a> {
    pub fn new(ctx: &'a mut Context, name: &'static str) -> Self {
        Self { ctx, shader_spirv: &[], pipeline_name: name }
    }

    pub fn shader(mut self, spirv: &'a [u32]) -> Self {
        self.shader_spirv = spirv;
        self
    }

    fn register_default_resources(
        ctx: &mut Context,
        descs: &HashMap<String, (usize, u32, u32)>,
        res: &mut ResourceManager,
    ) {
        for (name, def) in DEFAULT_RESOURCES.iter() {
            if descs.contains_key(&name.to_string()) {
                match def {
                    DefaultResource::Time => {
                        if res.get("KOJI_time").is_none() && res.get("time").is_none() {
                            res.register_time_buffers(ctx);
                            if let Some(ResourceBinding::Uniform(h)) = res.get("time") {
                                let handle = *h;
                                res.bindings.insert((*name).to_string(), ResourceBinding::Uniform(handle));
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_internal(
        self,
        mut res: Option<&mut ResourceManager>,
    ) -> Result<CPSO, PipelineError> {
        let info = reflect_shader(self.shader_spirv);
        let mut desc_map = HashMap::new();
        let mut bg_layouts: [Option<Handle<BindGroupLayout>>; 4] = [None, None, None, None];

        for set in info.bindings.keys().cloned().collect::<Vec<_>>() {
            let binds = &info.bindings[&set];
            let mut vars = Vec::new();

            for b in binds.iter() {
                if b.name.is_empty() {
                    panic!(
                        "Descriptor at set {} binding {} has no name. Provide an instance name in the shader source.",
                        set, b.binding
                    );
                }
                if desc_map.contains_key(&b.name) {
                    panic!(
                        "Descriptor name '{}' already used by another binding. Provide unique instance names in the shader source.",
                        b.name
                    );
                }

                let var_type = descriptor_to_var_type(b.ty);
                let mut count = b.count;
                if count == 0 {
                    if let Some(ref mut r) = res {
                        if let Some(binding_entry) = r.get(&b.name) {
                            count = match binding_entry {
                                ResourceBinding::TextureArray(arr) => arr.len() as u32,
                                ResourceBinding::CombinedTextureArray(arr) => arr.len() as u32,
                                ResourceBinding::BufferArray(arr) => arr.lock().unwrap().len() as u32,
                                _ => 0,
                            };
                        }
                    }
                    if count == 0 {
                        count = 1;
                    }
                }
                vars.push(BindGroupVariable {
                    var_type,
                    binding: b.binding,
                    count,
                });
                desc_map.insert(b.name.clone(), (set as usize, b.binding, count));
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

        if let Some(r) = res {
            Self::register_default_resources(self.ctx, &desc_map, r);
            for name in desc_map.keys() {
                if r.get(name).is_none() {
                    return Err(PipelineError::MissingResource(name.clone()));
                }
            }
        }

        let layout_info = ComputePipelineLayoutInfo {
            bg_layouts,
            shader: &PipelineShaderInfo {
                stage: ShaderType::Compute,
                spirv: self.shader_spirv,
                specialization: &[],
            },
        };

        let layout = self.ctx.make_compute_pipeline_layout(&layout_info).unwrap();

        let pipeline_handle = self
            .ctx
            .make_compute_pipeline(&ComputePipelineInfo {
                debug_name: self.pipeline_name,
                layout,
            })
            .unwrap();

        Ok(CPSO {
            pipeline: pipeline_handle,
            layout,
            bind_group_layouts: bg_layouts,
            desc_map,
            ctx: self.ctx,
        })
    }

    pub fn build(self) -> CPSO {
        self.build_internal(None).unwrap()
    }

    pub fn build_with_resources(self, res: &mut ResourceManager) -> Result<CPSO, PipelineError> {
        self.build_internal(Some(res))
    }
}

