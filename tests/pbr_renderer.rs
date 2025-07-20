use koji::material::*;
use koji::renderer::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use koji::render_pass::RenderPassBuilder;
use dashi::*;

use inline_spirv::include_spirv;
use koji::material::pipeline_builder::PipelineBuilder;
use dashi::utils::Handle;
fn make_vertex(pos: [f32;3], uv:[f32;2]) -> Vertex {
    Vertex {
        position: pos,
        normal: [0.0,0.0,1.0],
        tangent: [1.0,0.0,0.0,1.0],
        uv,
        color: [1.0,1.0,1.0,1.0],
    }
}

fn quad_vertices() -> Vec<Vertex> {
    vec![
        make_vertex([-0.5,-0.5,0.0],[0.0,0.0]),
        make_vertex([0.5,-0.5,0.0],[1.0,0.0]),
        make_vertex([0.5,0.5,0.0],[1.0,1.0]),
        make_vertex([-0.5,0.5,0.0],[0.0,1.0]),
    ]
}

fn quad_indices() -> Vec<u32> { vec![0,1,2,2,3,0] }

fn build_pbr_pipeline(ctx: &mut Context, rp: Handle<RenderPass>, subpass: u32) -> PSO {
    let vert: &[u32] = include_spirv!("assets/shaders/pbr.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("assets/shaders/pbr.frag", frag, glsl);
    PipelineBuilder::new(ctx, "pbr_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass((rp, subpass))
        .depth_enable(true)
        .cull_mode(CullMode::Back)
        .build()
}

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap().select(DeviceFilter::default().add_required_type(DeviceType::Dedicated)).unwrap_or_default();
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

    let mut renderer = Renderer::with_render_pass(320, 240, &mut ctx, builder).expect("renderer");
    renderer.add_canvas(canvas);

    let vert: &[u32] = include_spirv!("assets/shaders/pbr.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("assets/shaders/pbr.frag", frag, glsl);
    let mut pso = PipelineBuilder::new(&mut ctx, "pbr_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(graph.output("color"))
        .depth_enable(true)
        .cull_mode(CullMode::Back)
        .build();

    // register textures before creating bind groups
    let white: [u8;4] = [255,255,255,255];
    let img = ctx.make_image(&ImageInfo { debug_name:"alb", dim:[1,1,1], format:Format::RGBA8, mip_levels:1, layers:1, initial_data:Some(&white)}).unwrap();
    let view = ctx.make_image_view(&ImageViewInfo{ img, ..Default::default() }).unwrap();
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
    renderer.resources().register_combined("albedo_map", img, view,[1,1], sampler);
    renderer.resources().register_combined("normal_map", img, view,[1,1], sampler);
    renderer.resources().register_combined("metallic_map", img, view,[1,1], sampler);
    renderer.resources().register_combined("roughness_map", img, view,[1,1], sampler);

    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let mesh = StaticMesh {
        material_id: "pbr".into(),
        vertices: quad_vertices(),
        indices: Some(quad_indices()),
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh,None,"pbr".into());


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
    fn render_pbr_quad() {
        run();
    }
}
