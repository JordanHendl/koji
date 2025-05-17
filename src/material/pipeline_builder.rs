use crate::material::*;
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

/// Builder for a graphics pipeline, including reflection of SPIR-V
pub struct PipelineBuilder<'a> {
    ctx: &'a mut Context,
    vert_spirv: &'a [u32],
    frag_spirv: &'a [u32],
    render_pass: Option<Handle<RenderPass>>,
    pipeline_name: &'static str,
    subpass: u32,
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
        }
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
    pub fn build(self) -> Handle<GraphicsPipeline> {
        let rp = self
            .render_pass
            .expect("Render pass must be set before build");

        // Reflect descriptors from shaders
        let vert_info = reflect_shader(self.vert_spirv);
        let frag_info = reflect_shader(self.frag_spirv);

        // Merge descriptor sets across vert/frag
        let mut combined: HashMap<u32, Vec<ShaderDescriptorBinding>> = HashMap::new();
        for (set, binds) in vert_info.bindings.into_iter().chain(frag_info.bindings) {
            combined.entry(set).or_default().extend(binds);
        }

        // Build bind group layouts
        let mut bg_layouts: [Option<Handle<BindGroupLayout>>; 4] = [None, None, None, None];
        let mut sets: Vec<u32> = combined.keys().cloned().collect();
        sets.sort_unstable();
        for set in sets {
            let vars: Vec<BindGroupVariable> = combined[&set]
                .iter()
                .map(|b| BindGroupVariable {
                    var_type: descriptor_type_to_dashi(b.ty),
                    binding: b.binding,
                })
                .collect();
            let info = BindGroupLayoutInfo {
                debug_name: self.pipeline_name,
                shaders: &[ShaderInfo {
                    shader_type: ShaderType::All,
                    variables: &vars,
                }],
            };
            let layout = self
                .ctx
                .make_bind_group_layout(&info)
                .expect("layout failed");
            if (set as usize) < bg_layouts.len() {
                bg_layouts[set as usize] = Some(layout);
            } else {
                panic!("Descriptor set {} out of range", set);
            }
        }

        // Reflect vertex inputs
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

        // Assemble layout info
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
                culling: CullMode::Back,
                front_face: VertexOrdering::CounterClockwise,
                depth_test: Some(DepthInfo {
                    should_test: true,
                    should_write: true,
                }),
                ..Default::default()
            },
        };
        let layout = self
            .ctx
            .make_graphics_pipeline_layout(&layout_info)
            .unwrap();

        // Create pipeline
        let info = GraphicsPipelineInfo {
            debug_name: self.pipeline_name,
            layout,
            render_pass: rp,
            subpass_id: self.subpass as u8,
            ..Default::default()
        };
        self.ctx.make_graphics_pipeline(&info).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::pipeline_builder::ShaderDescriptorType;
    use dashi::builders::RenderPassBuilder;
    use inline_spirv::inline_spirv;
    use serial_test::serial;
    use spirv_reflect::types::ReflectFormat;
    
    fn make_ctx() -> Context {
        Context::headless(&ContextInfo::default()).unwrap()
    }
    
    fn simple_vertex_spirv() -> Vec<u32> {
        inline_spirv!(r#"
            #version 450
            layout(location=0) in vec2 pos;
            void main(){ gl_Position=vec4(pos,0,1);}"#
        , vert).to_vec()
    }
    fn simple_fragment_spirv() -> Vec<u32> {
        inline_spirv!(r#"
            #version 450
            layout(location=0) out vec4 outCol;
            void main(){ outCol=vec4(1); }"#
        , frag).to_vec()
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

        assert!(pipeline.valid());
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
        let vert = inline_spirv!(r#"
            #version 450
            layout(set=5,binding=0) uniform U{float x;};
            void main(){}
        "#, vert).to_vec();
        let frag = simple_fragment_spirv();

        // should panic on build
        let _ = PipelineBuilder::new(&mut ctx, "oops")
            .vertex_shader(&vert)
            .fragment_shader(&frag)
            .render_pass(rp, 0)
            .build();
    }
}

