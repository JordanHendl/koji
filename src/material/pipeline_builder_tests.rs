use std::sync::{Arc, Mutex};

use super::*;
use crate::{
    material::pipeline_builder::reflect_format_to_shader_primitive,
    shader_reflection::ShaderDescriptorType,
    utils::{
        allocator::GpuAllocator,
        resource_list::ResourceList,
        CombinedTextureSampler,
        DHObject,
        ResourceBuffer,
        ResourceBinding,
        Texture,
    },
};
use dashi::builders::RenderPassBuilder;
use inline_spirv::inline_spirv;
use serial_test::serial;
use spirv_reflect::types::ReflectFormat;

fn make_ctx() -> Context {
    Context::headless(&ContextInfo::default()).unwrap()
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
        descriptor_to_var_type(ShaderDescriptorType::SampledImage),
        BindGroupVariableType::SampledImage
    );
    assert_eq!(
        descriptor_to_var_type(ShaderDescriptorType::UniformBuffer),
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

    let variable = ShaderVariable::test_new(
        allocation,
        vec![("data".into(), 0, 4)],
        &mut ctx,
    );

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
    let variable = ShaderVariable::test_new(
        DHObject {
            handle: Handle::default(),
            offset: 0,
            size: 4,
        },
        vec![],
        std::ptr::null_mut(),
    );

    let mut resource = PSOResource::test_new(0, vec![("var1".into(), variable)]);

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

    let group = pso.create_bind_group(0, &resources).unwrap();

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
    let group = pso.create_bind_group(0, &resources).unwrap();

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
    resources.register_buffer_array("buf_array", Arc::new(Mutex::new(buf_array)));

    let group = pso.create_bind_group(0, &resources).unwrap();

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

fn multi_set_vert() -> Vec<u32> {
    inline_spirv!(
        r#"
        #version 450
        layout(location=0) in vec2 pos;
        layout(set=0, binding=0) uniform B0 { float x; } b0;
        void main() {
            gl_Position = vec4(pos, 0.0, 1.0);
        }
        "#,
        vert
    )
    .to_vec()
}

fn multi_set_frag() -> Vec<u32> {
    inline_spirv!(
        r#"
        #version 450
        layout(set=1, binding=0) uniform sampler2D tex;
        layout(location=0) out vec4 o;
        void main() {
            o = texture(tex, vec2(0.5));
        }
        "#,
        frag
    )
    .to_vec()
}

fn large_array_frag() -> Vec<u32> {
    inline_spirv!(
        r#"
        #version 450
        layout(set=0, binding=0) uniform sampler2D tex_array[];
        layout(location=0) out vec4 o;
        void main() {
            o = texture(tex_array[0], vec2(0.5));
        }
        "#,
        frag
    )
    .to_vec()
}

#[test]
#[serial]
fn create_bind_group_missing_resource() {
    let mut ctx = make_ctx();
    let rp = RenderPassBuilder::new("rp", Viewport::default())
        .add_subpass(&[AttachmentDescription::default()], None, &[])
        .build(&mut ctx)
        .unwrap();

    let mut pso = PipelineBuilder::new(&mut ctx, "missing")
        .vertex_shader(&simple_vert2())
        .fragment_shader(&simple_frag2())
        .render_pass(rp, 0)
        .build();

    let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();
    res.register_variable("b0", &mut ctx, 1u32);
    // intentionally omit "tex"

    match pso.create_bind_group(0, &res) {
        Err(PipelineError::MissingResource(name)) => assert_eq!(name, "tex"),
        _ => panic!("expected missing resource error"),
    }

    ctx.destroy();
}

#[test]
#[serial]
fn create_bind_groups_multiple_sets() {
    let mut ctx = make_ctx();
    let rp = RenderPassBuilder::new("rp", Viewport::default())
        .add_subpass(&[AttachmentDescription::default()], None, &[])
        .build(&mut ctx)
        .unwrap();

    let mut pso = PipelineBuilder::new(&mut ctx, "multi_set")
        .vertex_shader(&multi_set_vert())
        .fragment_shader(&multi_set_frag())
        .render_pass(rp, 0)
        .build();

    let mut res = ResourceManager::new(&mut ctx, 2048).unwrap();
    res.register_variable("b0", &mut ctx, 55u32);
    let img = ctx.make_image(&ImageInfo::default()).unwrap();
    let view = ctx
        .make_image_view(&ImageViewInfo { img, ..Default::default() })
        .unwrap();
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
    res.register_combined("tex", img, view, [1, 1], sampler);

    let sets = pso.create_bind_groups(&res).unwrap();
    assert!(sets[0].is_some());
    assert!(sets[1].is_some());
    assert!(sets[2].is_none());
    assert!(sets[3].is_none());

    let set0 = sets[0].as_ref().unwrap();
    assert!(set0.buffers.contains_key("b0"));
    let set1 = sets[1].as_ref().unwrap();
    assert!(set1.textures.contains_key("tex"));

    ctx.destroy();
}

#[test]
#[serial]
fn auto_register_time_buffer() {
    let mut ctx = make_ctx();
    let rp = RenderPassBuilder::new("rp", Viewport::default())
        .add_subpass(&[AttachmentDescription::default()], None, &[])
        .build(&mut ctx)
        .unwrap();

    let vert = simple_vertex_spirv();
    let frag = inline_spirv!(
        r#"
        #version 450
        layout(set=0, binding=0) uniform Time { vec2 t; } time;
        layout(location=0) out vec4 o;
        void main(){ o = vec4(time.t, 0.0, 1.0); }
        "#,
        frag
    )
    .to_vec();

    let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();

    let _pso = PipelineBuilder::new(&mut ctx, "time_test")
        .vertex_shader(&vert)
        .fragment_shader(&frag)
        .render_pass(rp, 0)
        .resources(&mut res)
        .build();

    match res.get("time") {
        Some(ResourceBinding::Uniform(_)) => {}
        _ => panic!("time buffer not registered"),
    }

    ctx.destroy();
}

#[cfg(feature = "large-tests")]
#[test]
#[serial]
#[ignore]
fn large_indexed_array_bindings() {
    let mut ctx = make_ctx();
    let rp = RenderPassBuilder::new("rp", Viewport::default())
        .add_subpass(&[AttachmentDescription::default()], None, &[])
        .build(&mut ctx)
        .unwrap();

    let mut pso = PipelineBuilder::new(&mut ctx, "large_array")
        .vertex_shader(&simple_vertex_spirv())
        .fragment_shader(&large_array_frag())
        .render_pass(rp, 0)
        .build();

    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
    let mut tex_array = ResourceList::<CombinedTextureSampler>::default();
    // allocate a moderately large array of textures to stress descriptor
    // handling without exceeding device limits
    for _ in 0..16 {
        let img = ctx.make_image(&ImageInfo::default()).unwrap();
        let view = ctx
            .make_image_view(&ImageViewInfo { img, ..Default::default() })
            .unwrap();
        tex_array.push(CombinedTextureSampler {
            texture: Texture { handle: img, view, dim: [32, 32] },
            sampler,
        });
    }
    let mut res = ResourceManager::new(&mut ctx, 4096).unwrap();
    res.register_combined_texture_array("tex_array", Arc::new(tex_array));

    let group = pso.create_bind_group(0, &res).unwrap();
    assert!(group.bind_group.valid());

    ctx.destroy();
}

