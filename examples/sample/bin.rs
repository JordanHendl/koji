use dashi::*;
use inline_spirv::include_spirv;
use koji::material::PipelineBuilder;
use koji::renderer::*;

pub fn run(ctx: &mut Context) {
    let mut renderer = Renderer::new(640, 480, "sample", ctx).unwrap();

    let vert: &[u32] = include_spirv!("assets/shaders/sample.vert", vert);
    let frag: &[u32] = include_spirv!("assets/shaders/sample.frag", frag);

    let tex_data: [u8; 12] = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
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

    renderer
        .resources()
        .register_combined("tex", img, view, [3, 1], sampler);
    renderer
        .resources()
        .register_variable("ubo", ctx, 0.7f32);

    let mut pso = PipelineBuilder::new(ctx, "sample_pso")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.render_pass(), 0)
        .build_with_resources(renderer.resources())
        .unwrap();

    let bind_groups = pso
        .create_bind_groups(renderer.resources())
        .unwrap();
    renderer.register_pipeline_for_pass("main", pso, bind_groups);

    let mesh = StaticMesh {
        material_id: "color".into(),
        vertices: vec![
            Vertex {
                position: [0.0, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh, None, "color".into());

    renderer.render_loop(|_r, _event| {});
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
