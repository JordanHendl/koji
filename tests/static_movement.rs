use koji::material::*;
use koji::renderer::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use koji::render_pass::RenderPassBuilder;
use dashi::*;

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

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();

    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let builder = RenderPassBuilder::new()
        .debug_name("MainPass")
        .color_attachment("color", Format::RGBA8)
        .subpass("main", ["color"], &[] as &[&str]);

    let mut renderer = Renderer::with_render_pass(320, 240, &mut ctx, builder).unwrap();
    renderer.add_canvas(canvas);

    let mut pso = PipelineBuilder::new(&mut ctx,"move_pso")
        .vertex_shader(&vert())
        .fragment_shader(&frag())
        .render_pass(graph.output("color"))
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let mesh = StaticMesh {
        material_id: "default".into(),
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
    renderer.register_static_mesh(mesh,None,"default".into());

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

#[cfg(all(test, feature = "gpu_tests"))]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    #[ignore]
    fn static_mesh_with_movement() {
        run();
    }
}
