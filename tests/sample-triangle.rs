use koji::*;
use koji::renderer::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use koji::render_pass::RenderPassBuilder;
use dashi::*;
use inline_spirv::include_spirv;
use serial_test::serial;
// External shader files are loaded from `assets/shaders/` using `include_spirv!`.

fn make_color_vertex(position: [f32; 3], color: [f32; 4]) -> Vertex {
    Vertex {
        position,
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
        color,
    }
}

fn triangle_vertices() -> Vec<Vertex> {
    vec![
        make_color_vertex([0.0, -0.5, 0.0], [1.0, 0.0, 0.0, 1.0]),
        make_color_vertex([0.5, 0.5, 0.0], [0.0, 1.0, 0.0, 1.0]),
        make_color_vertex([-0.5, 0.5, 0.0], [0.0, 0.0, 1.0, 1.0]),
    ]
}

fn cube_vertices() -> Vec<Vertex> {
    vec![
        // Front face
        make_color_vertex([-0.5, -0.5,  0.5], [1.0, 0.0, 0.0, 1.0]),
        make_color_vertex([ 0.5, -0.5,  0.5], [0.0, 1.0, 0.0, 1.0]),
        make_color_vertex([ 0.5,  0.5,  0.5], [0.0, 0.0, 1.0, 1.0]),
        make_color_vertex([-0.5,  0.5,  0.5], [1.0, 1.0, 0.0, 1.0]),
        // Back face
        make_color_vertex([-0.5, -0.5, -0.5], [1.0, 0.0, 1.0, 1.0]),
        make_color_vertex([ 0.5, -0.5, -0.5], [0.0, 1.0, 1.0, 1.0]),
        make_color_vertex([ 0.5,  0.5, -0.5], [1.0, 1.0, 1.0, 1.0]),
        make_color_vertex([-0.5,  0.5, -0.5], [0.2, 0.5, 0.8, 1.0]),
    ]
}

fn cube_indices() -> Vec<u32> {
    vec![
        // Front
        0, 1, 2, 2, 3, 0,
        // Right
        1, 5, 6, 6, 2, 1,
        // Back
        7, 6, 5, 5, 4, 7,
        // Left
        4, 0, 3, 3, 7, 4,
        // Top
        3, 2, 6, 6, 7, 3,
        // Bottom
        4, 5, 1, 1, 0, 4,
    ]
}

fn make_shader_vert() -> Vec<u32> {
    include_spirv!(
        "assets/shaders/test_triangle.vert",
        vert
    )
    .to_vec()
}

fn make_shader_frag() -> Vec<u32> {
    include_spirv!(
        "assets/shaders/test_triangle.frag",
        frag
    )
    .to_vec()
}

#[test]
#[serial]
#[ignore]
fn render_triangle_and_cube() {
    // Set up device/context and resource manager
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();

    let canvas = CanvasBuilder::new()
        .extent([640, 480])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let builder = RenderPassBuilder::new()
        .debug_name("MainPass")
        .color_attachment("color", Format::RGBA8)
        .subpass("main", ["color"], &[] as &[&str]);

    let mut renderer = Renderer::with_render_pass(640, 480, &mut ctx, builder).expect("Error making Renderer");
    renderer.add_canvas(canvas);

    // Shaders
    let vert_spv = make_shader_vert();
    let frag_spv = make_shader_frag();

    // Create pipeline (PSO) using your PipelineBuilder
    let mut pso = PipelineBuilder::new(&mut ctx, "triangle_cube_pipeline")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass(renderer.graph().output("color"))
        .build();

    // Generate/cached bind group resources for all sets
    let bind_group_resources = pso.create_bind_groups(&renderer.resources()).unwrap();

    // Register pipeline+resources
    renderer.register_pipeline_for_pass("main", pso, bind_group_resources);

    // Register triangle
    let triangle_mesh = StaticMesh {
        material_id: "color".into(),
        vertices: triangle_vertices(),
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(triangle_mesh, None, "color".into());

    // Register cube
    let cube_mesh = StaticMesh {
        material_id: "color".into(),
        vertices: cube_vertices(),
        indices: Some(cube_indices()),
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(cube_mesh, None, "color".into());

    // Main loop: just draw both objects with same pipeline/PSO/bind group
    renderer.render_loop(|_r, _event| {
        // Nothing to update per frame in this simple test
    });
}

