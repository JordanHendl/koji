use dashi::*;
use inline_spirv::include_spirv;
use koji::renderer::*;
use koji::render_pass::*;
use koji::material::*;
use serde_yaml;

pub fn run(ctx: &mut Context) {
    // YAML description for a render pass with two subpasses
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

    let config: YamlRenderPass = serde_yaml::from_str(yaml).unwrap();
    let builder = RenderPassBuilder::from_yaml(config);

    let mut renderer = Renderer::with_render_pass(640, 480, ctx, builder).unwrap();

    // Shaders for a colored triangle
    let vert = include_spirv!("assets/shaders/test_triangle.vert", vert);
    let frag = include_spirv!("assets/shaders/test_triangle.frag", frag);

    // Pipeline for first subpass
    let mut pso_first = PipelineBuilder::new(ctx, "first_pso")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.render_pass(), 0)
        .build();
    let bgr_first = pso_first.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("first", pso_first, bgr_first);

    // Pipeline for second subpass
    let mut pso_second = PipelineBuilder::new(ctx, "second_pso")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.render_pass(), 1)
        .build();
    let bgr_second = pso_second.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("second", pso_second, bgr_second);

    // Simple triangle mesh
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

    // Draw a single frame
    renderer.present_frame().unwrap();
}

pub fn main() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
    run(&mut ctx);
    ctx.destroy();
}
