#![cfg(feature = "gpu_tests")]

use dashi::gpu;
use dashi::*;
use inline_spirv::include_spirv;
use koji::*;
use koji::renderer::test_hooks::take_draw_events;
use serial_test::serial;

fn setup_ctx() -> gpu::Context {
    gpu::Context::headless(&Default::default()).unwrap()
}

fn simple_mesh() -> StaticMesh {
    StaticMesh {
        material_id: String::new(),
        vertices: vec![
            Vertex{position:[0.0,-0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,0.0],color:[1.0,0.0,0.0,1.0]},
            Vertex{position:[0.5,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[1.0,1.0],color:[0.0,1.0,0.0,1.0]},
            Vertex{position:[-0.5,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,1.0],color:[0.0,0.0,1.0,1.0]},
        ],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    }
}

fn execute_graph(ctx: &mut gpu::Context, graph: RenderGraph) -> Vec<String> {
    let mut renderer = Renderer::with_graph_headless(1, 1, ctx, graph).unwrap();
    let vert = include_spirv!("assets/shaders/test_triangle.vert", vert);
    let frag = include_spirv!("assets/shaders/test_triangle.frag", frag);

    let mut pso_first = PipelineBuilder::new(ctx, "pso_first")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.graph().output("first"))
        .build();
    let bgr_first = pso_first.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_material_pipeline("mat_first", pso_first, bgr_first);

    let mut pso_second = PipelineBuilder::new(ctx, "pso_second")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.graph().output("second"))
        .build();
    let bgr_second = pso_second.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_material_pipeline("mat_second", pso_second, bgr_second);

    let mesh1 = simple_mesh();
    let mesh2 = simple_mesh();
    renderer.register_static_mesh(mesh1, None, "mat_first".into(), "first");
    renderer.register_static_mesh(mesh2, None, "mat_second".into(), "second");

    renderer.present_frame().unwrap();
    take_draw_events()
}

#[test]
#[serial]
fn yaml_graph_roundtrip_execution() {
    let yaml = r#"nodes:
  - name: first
    inputs: []
    outputs:
      - name: first
        format: RGBA8
  - name: second
    inputs:
      - name: first
        format: RGBA8
    outputs:
      - name: second
        format: RGBA8
  - name: third
    inputs:
      - name: second
        format: RGBA8
    outputs:
      - name: third
        format: RGBA8
canvases:
  - name: first
    canvas:
      extent: [1, 1]
      attachments:
        - name: first
          format: RGBA8
  - name: second
    canvas:
      extent: [1, 1]
      attachments:
        - name: second
          format: RGBA8
  - name: third
    canvas:
      extent: [1, 1]
      attachments:
        - name: third
          format: RGBA8
edges:
  - [first, second]
  - [second, third]
"#;

    let mut ctx = setup_ctx();
    let graph = koji::render_graph::from_yaml(&mut ctx, yaml).unwrap();
    let yaml_rt = koji::render_graph::to_yaml(&graph).unwrap();
    let graph_rt = koji::render_graph::from_yaml(&mut ctx, &yaml_rt).unwrap();
    let events = execute_graph(&mut ctx, graph_rt);
    assert_eq!(
        events,
        vec![
            "pass:first",
            "begin_static",
            "end_static",
            "pass:second",
            "begin_static",
            "end_static",
            "pass:third",
        ]
    );
    ctx.destroy();
}

#[test]
#[serial]
fn json_graph_roundtrip_execution() {
    let json = r#"{
  "nodes": [
    {"name": "first", "inputs": [], "outputs": [{"name": "first", "format": "RGBA8"}]},
    {"name": "second", "inputs": [{"name": "first", "format": "RGBA8"}], "outputs": [{"name": "second", "format": "RGBA8"}]},
    {"name": "third", "inputs": [{"name": "second", "format": "RGBA8"}], "outputs": [{"name": "third", "format": "RGBA8"}]}
  ],
  "canvases": [
    {"name": "first", "canvas": {"extent": [1,1], "attachments": [{"name":"first","format":"RGBA8"}]}},
    {"name": "second", "canvas": {"extent": [1,1], "attachments": [{"name":"second","format":"RGBA8"}]}},
    {"name": "third", "canvas": {"extent": [1,1], "attachments": [{"name":"third","format":"RGBA8"}]}}
  ],
  "edges": [["first","second"],["second","third"]]
}"#;

    let mut ctx = setup_ctx();
    let graph = koji::render_graph::from_json(&mut ctx, json).unwrap();
    let json_rt = koji::render_graph::to_json(&graph).unwrap();
    let graph_rt = koji::render_graph::from_json(&mut ctx, &json_rt).unwrap();
    let events = execute_graph(&mut ctx, graph_rt);
    assert_eq!(
        events,
        vec![
            "pass:first",
            "begin_static",
            "end_static",
            "pass:second",
            "begin_static",
            "end_static",
            "pass:third",
        ]
    );
    ctx.destroy();
}
