use koji::material::*;
use koji::renderer::*;
use koji::utils::*;
use dashi::*;
use serial_test::serial;
use inline_spirv::inline_spirv;

fn vert() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(location=0) in vec3 pos;
        void main(){ gl_Position=vec4(pos,1.0); }",
        vert
    ).to_vec()
}

fn frag() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(set=0,binding=0) buffer Lights { vec4 data[]; };
        layout(location=0) out vec4 o;
        void main(){ o = data.length()>0 ? data[0] : vec4(1.0); }",
        frag
    ).to_vec()
}

#[test]
#[serial]
#[ignore]
fn bindless_lighting_sample() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"lights", &mut ctx).unwrap();

    let mut pso = PipelineBuilder::new(&mut ctx, "lights")
        .vertex_shader(&vert())
        .fragment_shader(&frag())
        .render_pass(renderer.render_pass(),0)
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pso(RenderStage::Opaque, pso, bgr);

    let mut lights = BindlessLights::new();
    let light = LightDesc{ position:[0.0,0.0,0.0], intensity:1.0, color:[1.0,1.0,1.0], range:1.0, direction:[0.0,0.0,-1.0], _pad:0 };
    lights.add_light(&mut ctx, renderer.resources(), light);
    lights.register(renderer.resources());

    let mesh = StaticMesh {
        vertices: vec![
            Vertex{position:[-0.5,-0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,0.0],color:[1.0,1.0,1.0,1.0]},
            Vertex{position:[0.5,-0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[1.0,0.0],color:[1.0,1.0,1.0,1.0]},
            Vertex{position:[0.0,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.5,1.0],color:[1.0,1.0,1.0,1.0]},
        ],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh,None);

    renderer.present_frame().unwrap();
    ctx.destroy();
}
