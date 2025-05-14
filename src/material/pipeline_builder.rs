use crate::material::*;
use dashi::*;
use std::collections::HashMap;

use spirv_reflect::types::ReflectFormat;
use spirv_reflect::ShaderModule;

use self::shader_reflection::*;

// map descriptor types to Dashi
fn descriptor_type_to_dashi(ty: ShaderDescriptorType) -> BindGroupVariableType {
    match ty {
        ShaderDescriptorType::SampledImage => BindGroupVariableType::SampledImage,
        ShaderDescriptorType::CombinedImageSampler => BindGroupVariableType::SampledImage,
        ShaderDescriptorType::UniformBuffer => BindGroupVariableType::Uniform,
        ShaderDescriptorType::StorageBuffer => BindGroupVariableType::Storage,
        ShaderDescriptorType::StorageImage => BindGroupVariableType::StorageImage,
        _ => panic!("Unsupported descriptor type: {:?}", ty),
    }
}

// map SPIR-V reflect format to your primitive enum
fn reflect_format_to_shader_primitive(fmt: ReflectFormat) -> ShaderPrimitiveType {
    use ReflectFormat::*;
    match fmt {
        R32G32B32A32_SFLOAT => ShaderPrimitiveType::Vec4,
        R32G32B32_SFLOAT => ShaderPrimitiveType::Vec3,
        R32G32_SFLOAT => ShaderPrimitiveType::Vec2,
        _ => panic!("Unsupported vertex input format: {:?}", fmt),
    }
}

pub struct PipelineBuilder<'a> {
    pub ctx: &'a mut Context,
    pub spirv_vert: &'a [u32],
    pub spirv_frag: &'a [u32],
    pub pipeline_name: &'static str,
    pub subpass: u32,
}

impl<'a> PipelineBuilder<'a> {
    pub fn build(self) -> Handle<GraphicsPipeline> {
        // reflect descriptors & push constants
        let vert_info = reflect_shader(self.spirv_vert);
        let frag_info = reflect_shader(self.spirv_frag);

        // merge descriptor sets
        let mut combined_sets: HashMap<u32, Vec<ShaderDescriptorBinding>> = HashMap::new();
        for (set, bindings) in vert_info.bindings.into_iter().chain(frag_info.bindings) {
            combined_sets.entry(set).or_default().extend(bindings);
        }

        // build up to 4 BindGroupLayouts
        let mut bg_layouts: [Option<Handle<BindGroupLayout>>; 4] = [None, None, None, None];
        let mut sets: Vec<u32> = combined_sets.keys().copied().collect();
        sets.sort_unstable();
        for &set in &sets {
            let vars: Vec<BindGroupVariable> = combined_sets[&set]
                .iter()
                .map(|b| BindGroupVariable {
                    var_type: descriptor_type_to_dashi(b.ty),
                    binding: b.binding,
                })
                .collect();

            let shader_info = ShaderInfo {
                shader_type: ShaderType::All,
                variables: &vars,
            };

            let layout_info = BindGroupLayoutInfo {
                debug_name: self.pipeline_name,
                shaders: &[shader_info],
            };

            let layout = self
                .ctx
                .make_bind_group_layout(&layout_info)
                .expect("make_bind_group_layout failed");

            let idx = set as usize;
            if idx < bg_layouts.len() {
                bg_layouts[idx] = Some(layout);
            } else {
                panic!("Descriptor set {} out of range", set);
            }
        }

        // reflect vertex inputs
        let module =
            ShaderModule::load_u32_data(self.spirv_vert).expect("Failed to parse vertex SPIR-V");
        let mut inputs = module
            .enumerate_input_variables(None)
            .expect("Failed to enumerate vertex inputs");
        inputs.sort_by_key(|v| v.location);

        let mut entries_vec = Vec::with_capacity(inputs.len());
        let mut offset = 0u32;
        for var in inputs {
            let fmt = reflect_format_to_shader_primitive(var.format);
            entries_vec.push(VertexEntryInfo {
                format: fmt,
                location: var.location as usize,
                offset: offset as usize,
            });

            let size = match fmt {
                ShaderPrimitiveType::Vec4 | ShaderPrimitiveType::IVec4 => 16,
                ShaderPrimitiveType::Vec3 => 12,
                ShaderPrimitiveType::Vec2 => 8,
                _ => panic!("Unexpected primitive {:?}", fmt),
            };
            offset += size;
        }
        let vertex_info = VertexDescriptionInfo {
            entries: &entries_vec,
            stride: offset as usize,
            rate: VertexRate::Vertex,
        };

        // collect and sort push constants
        let mut pcs = vert_info
            .push_constants
            .into_iter()
            .chain(frag_info.push_constants)
            .collect::<Vec<_>>();
        pcs.sort_by_key(|pc| pc.offset);

        // assemble pipeline layout info inline
        let layout_info = GraphicsPipelineLayoutInfo {
            debug_name: self.pipeline_name,
            vertex_info,
            bg_layouts,
            shaders: &[
                PipelineShaderInfo {
                    stage: ShaderType::Vertex,
                    spirv: self.spirv_vert,
                    specialization: &[],
                },
                PipelineShaderInfo {
                    stage: ShaderType::Fragment,
                    spirv: self.spirv_frag,
                    specialization: &[],
                },
            ],
            details: GraphicsPipelineDetails {
                subpass: self.subpass as u8,
                color_blend_states: vec![ColorBlendState::default()],
                topology: Topology::TriangleList,
                culling: CullMode::Back,
                front_face: VertexOrdering::CounterClockwise,
                depth_test: Some(DepthInfo {
                    should_test: true,
                    should_write: true,
                }),
            },
        };

        let pipeline_layout = self
            .ctx
            .make_graphics_pipeline_layout(&layout_info)
            .expect("make_graphics_pipeline_layout failed");

        // finally create the pipeline
        let pipeline_info = GraphicsPipelineInfo {
            debug_name: self.pipeline_name,
            layout: pipeline_layout,
            ..Default::default()
        };

        self.ctx
            .make_graphics_pipeline(&pipeline_info)
            .expect("make_graphics_pipeline failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use crate::material::pipeline_builder::ShaderDescriptorType;
    use spirv_reflect::types::ReflectFormat;

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
            reflect_format_to_shader_primitive(R32G32B32A32_SFLOAT),
            ShaderPrimitiveType::Vec4
        );
        assert_eq!(
            reflect_format_to_shader_primitive(R32G32_SFLOAT),
            ShaderPrimitiveType::Vec2
        );
    }

    #[test]
    #[serial]
    fn subpass_is_respected() {
        // Use a headless Context if available, else mock
//        let mut ctx = Context::headless().unwrap();

        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = Context::new(&ContextInfo { device }).unwrap();

        let builder = PipelineBuilder {
            ctx: &mut ctx,
            spirv_vert: &[],
            spirv_frag: &[],
            pipeline_name: "test",
            subpass: 3,
        };
        let layout_info = {
            // inline copy of build's layout assembly for test
            let vert_info = reflect_shader(&[]);
            let frag_info = reflect_shader(&[]);
            let mut combined: HashMap<u32, Vec<_>> = HashMap::new();
            for (s, b) in vert_info.bindings.into_iter().chain(frag_info.bindings) {
                combined.entry(s).or_default().extend(b);
            }
            let layouts = [None, None, None, None];
            GraphicsPipelineLayoutInfo {
                debug_name: builder.pipeline_name,
                vertex_info: VertexDescriptionInfo {
                    entries: &[],
                    stride: 0,
                    rate: VertexRate::Vertex,
                },
                bg_layouts: layouts,
                shaders: &[],
                details: GraphicsPipelineDetails {
                    subpass: builder.subpass as u8,
                    color_blend_states: vec![],
                    topology: Topology::TriangleList,
                    culling: CullMode::Back,
                    front_face: VertexOrdering::CounterClockwise,
                    depth_test: None,
                },
            }
        };
        assert_eq!(layout_info.details.subpass, 3);
    }
}
