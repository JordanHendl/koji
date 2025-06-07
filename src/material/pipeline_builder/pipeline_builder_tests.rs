#[cfg(test)]
mod tests {
    use crate::*;
    use crate::utils::*;
    use crate::material::*;
    use std::sync::Arc;
    use dashi::builders::RenderPassBuilder;
    use dashi::{AttachmentDescription, ContextInfo, Viewport};
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
        gpu::Context::headless(&Default::default()).unwrap()
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
            layout(set=0, binding=0) uniform sampler2D bindless_textures[];
            layout(location=0) out vec4 o;
            void main() {
                // Index 2 for test, would be dynamic in real shaders
                o = texture(bindless_textures[2], vec2(0.5));
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
        resources.register_combined_texture_array("bindless_textures", tex_array.clone());

        // The pipeline should reflect the unsized array and request the bindless resource
        let group = pso.create_bind_group(0, &resources);

        // Expect a valid bind group, and that "bindless_textures" is registered as a texture array
        assert!(group.bind_group.valid());
        assert!(resources.get("bindless_textures").is_some());

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

