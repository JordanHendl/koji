use dashi::*;
use inline_spirv::include_spirv;
use koji::renderer::*;
use koji::material::*;
use koji::ResourceBinding;

pub fn run(ctx: &mut Context) {
    // Initialize renderer
    let mut renderer = Renderer::new(640, 480, "sample", ctx).unwrap();

    // Compile shaders
    let vert = include_spirv!("assets/shaders/sample.vert", vert);
    let frag = include_spirv!("assets/shaders/sample.frag", frag);

    // Build pipeline and auto-register timing resource
    let mut pso = PipelineBuilder::new(ctx, "sample_pso")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.render_pass(), 0)
        .build_with_resources(renderer.resources());

    // Texture and uniform
    let tex_data: [u8; 12] = [
        255, 0, 0, 255,
        0, 255, 0, 255,
        0, 0, 255, 255,
    ];
    let img = ctx
        .make_image(&ImageInfo {
            debug_name: "sample_tex",
            dim: [3, 1, 1],
            format: Format::RGBA8,
            mip_levels: 1,
            layers: 1,
            initial_data: Some(&tex_data),
        })
        .unwrap();
    let view = ctx
        .make_image_view(&ImageViewInfo {
            img,
            debug_name: "sample_tex_view",
            ..Default::default()
        })
        .unwrap();
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();

    let res = renderer.resources();
    res.register_combined("tex", img, view, [1, 1], sampler);
    res.register_variable("ubo", ctx, 0.7f32);
    if res.get("KOJI_time").is_none() {
        if let Some(ResourceBinding::Uniform(h)) = res.get("time") {
            res.register_ubo("KOJI_time", *h);
        }
    }

    // Create bind groups
    let bgr = pso.create_bind_groups(res).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    // Triangle mesh
    let mesh = StaticMesh {
        material_id: "sample".into(),
        vertices: vec![
            Vertex { position: [0.0, -0.5, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 0.0], color: [1.0, 0.0, 0.0, 1.0] },
            Vertex { position: [0.5, 0.5, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [1.0, 1.0], color: [0.0, 1.0, 0.0, 1.0] },
            Vertex { position: [-0.5, 0.5, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 1.0], color: [0.0, 0.0, 1.0, 1.0] },
        ],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh, None, "sample".into());

    // Draw loop
    renderer.render_loop(|_| {});
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
