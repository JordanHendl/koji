use dashi::*;
use inline_spirv::include_spirv;
use koji::renderer::*;
use koji::render_pass::*;
use koji::material::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use serde_yaml;

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();

    let yaml = r#"
debug_name: custom_pass
attachments:
  - name: first
    format: RGBA8
  - name: second
    format: RGBA8
subpasses:
  - name: first
    color_attachments: [first]
    depends_on: []
  - name: second
    color_attachments: [second]
    depends_on: [first]
"#;

    let canvas = CanvasBuilder::new()
        .extent([640, 480])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    let config: YamlRenderPass = serde_yaml::from_str(yaml).unwrap();
    graph.add_node::<RenderPassBuilderNode>(RenderPassBuilder::from_yaml(config).into());
    graph.add_canvas(&canvas);

    let mut renderer = Renderer::with_graph(640, 480, &mut ctx, graph).unwrap();

    let vert = include_spirv!("assets/shaders/test_triangle.vert", vert);
    let frag = include_spirv!("assets/shaders/test_triangle.frag", frag);

    let mut pso_first = PipelineBuilder::new(&mut ctx, "first_pso")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.graph().output("first"))
        .build();
    let bgr_first = pso_first.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("first", pso_first, bgr_first);

    let mut pso_second = PipelineBuilder::new(&mut ctx, "second_pso")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.graph().output("second"))
        .build();
    let bgr_second = pso_second.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("second", pso_second, bgr_second);

    let mesh = StaticMesh {
        material_id: "color".into(),
        vertices: vec![
            Vertex{position:[0.0,-0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,0.0],color:[1.0,0.0,0.0,1.0]},
            Vertex{position:[0.5,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[1.0,1.0],color:[0.0,1.0,0.0,1.0]},
            Vertex{position:[-0.5,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,1.0],color:[0.0,0.0,1.0,1.0]},
        ],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh, None, "color".into());

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
    fn custom_render_pass() {
        run();
    }
}

