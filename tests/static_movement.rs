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
        void main(){ gl_Position = vec4(pos,1.0); }",
        vert
    ).to_vec()
}
fn frag() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(location=0) out vec4 o;
        void main(){ o=vec4(1.0); }",
        frag
    ).to_vec()
}

fn make_vertex(pos:[f32;3]) -> Vertex {
    Vertex { position:pos, normal:[0.0,0.0,1.0], tangent:[1.0,0.0,0.0,1.0], uv:[0.0,0.0], color:[1.0,1.0,1.0,1.0] }
}

#[test]
#[serial]
#[ignore]
fn static_mesh_with_movement() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"move", &mut ctx).unwrap();

    let mut pso = PipelineBuilder::new(&mut ctx,"move_pso")
        .vertex_shader(&vert())
        .fragment_shader(&frag())
        .render_pass(renderer.render_pass(),0)
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pso(RenderStage::Opaque,pso,bgr);

    let mut mesh = StaticMesh {
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

    // move vertices slightly
    let new_verts = vec![
        make_vertex([-0.25,-0.25,0.0]),
        make_vertex([0.75,-0.25,0.0]),
        make_vertex([0.25,0.75,0.0]),
    ];
    renderer.update_static_mesh(0,&new_verts);

    renderer.present_frame().unwrap();
    ctx.destroy();
}
