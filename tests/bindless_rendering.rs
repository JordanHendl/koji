use koji::material::*;
use koji::renderer::*;
use dashi::*;

use inline_spirv::inline_spirv;

fn simple_vert() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(location=0) in vec3 pos;
        void main(){ gl_Position = vec4(pos,1.0); }",
        vert
    ).to_vec()
}

fn simple_frag() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(location=0) out vec4 o;
        void main(){ o = vec4(1.0); }",
        frag
    ).to_vec()
}

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"bindless", &mut ctx).unwrap();

    let vert = simple_vert();
    let frag = simple_frag();
    let mut pso = PipelineBuilder::new(&mut ctx, "bindless")
        .vertex_shader(&vert)
        .fragment_shader(&frag)
        .render_pass(renderer.render_pass(),0)
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let mut bindless = BindlessData::new();
    let tex_data:[u8;4] = [255,0,0,255];
    let img = ctx.make_image(&ImageInfo{debug_name:"t",dim:[1,1,1],format:Format::RGBA8,mip_levels:1,layers:1,initial_data:Some(&tex_data)}).unwrap();
    let view = ctx.make_image_view(&ImageViewInfo{img,..Default::default()}).unwrap();
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
    bindless.add_texture(img, view, sampler, [1,1]);
    #[repr(C)]
    #[derive(Clone,Copy,bytemuck::Pod,bytemuck::Zeroable)]
    struct Dummy{val:f32}
    let _ = bindless.add_material(&mut ctx, renderer.resources(), Dummy{val:1.0});
    bindless.register(renderer.resources());

    let mesh = StaticMesh {
        material_id: "bindless".into(),
        vertices: vec![
            Vertex{position:[0.0,-0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,0.0],color:[1.0,1.0,1.0,1.0]},
            Vertex{position:[0.5,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[1.0,1.0],color:[1.0,1.0,1.0,1.0]},
            Vertex{position:[-0.5,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,1.0],color:[1.0,1.0,1.0,1.0]},
        ],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh,None,"bindless".into());
    renderer.present_frame().unwrap();
    ctx.destroy();
}

#[cfg(all(test, feature = "gpu_tests"))]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    #[ignore]
    fn bindless_rendering_sample() {
        run();
    }
}
