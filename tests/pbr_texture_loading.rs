use koji::material::*;
use koji::renderer::*;
use koji::texture_manager as texman;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use dashi::*;
use dashi::gpu;
use dashi::utils::Handle;
use inline_spirv::include_spirv;
use image::{ImageBuffer, Rgba, ImageOutputFormat};
use std::io::Cursor;

fn make_vertex(pos: [f32; 3], uv: [f32; 2]) -> Vertex {
    Vertex {
        position: pos,
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 0.0, 1.0],
        uv,
        color: [1.0, 1.0, 1.0, 1.0],
    }
}

fn quad_vertices() -> Vec<Vertex> {
    vec![
        make_vertex([-0.5, -0.5, 0.0], [0.0, 0.0]),
        make_vertex([0.5, -0.5, 0.0], [1.0, 0.0]),
        make_vertex([0.5, 0.5, 0.0], [1.0, 1.0]),
        make_vertex([-0.5, 0.5, 0.0], [0.0, 1.0]),
    ]
}

fn quad_indices() -> Vec<u32> {
    vec![0, 1, 2, 2, 3, 0]
}

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

fn png_bytes(color: [u8; 4]) -> Vec<u8> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(1, 1, Rgba(color));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageOutputFormat::Png).unwrap();
    buf
}

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let mut ctx = gpu::Context::headless(&Default::default()).unwrap();

    let canvas = CanvasBuilder::new()
        .extent([64, 64])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let mut renderer = Renderer::with_graph(64, 64, &mut ctx, graph).unwrap();

    let vert: &[u32] = include_spirv!("assets/shaders/pbr.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("assets/shaders/pbr.frag", frag, glsl);
    let mut pso = PipelineBuilder::new(&mut ctx, "pbr_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.graph().output("color"))
        .depth_enable(true)
        .cull_mode(CullMode::Back)
        .build();

    let colors = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];

    let keys = ["albedo_map", "normal_map", "metallic_map", "roughness_map"];
    let mut handles = Vec::new();
    for (key, col) in keys.iter().zip(colors.iter()) {
        let bytes = png_bytes(*col);
        let handle = texman::load_from_bytes(&mut ctx, renderer.resources(), key, Default::default(), &bytes);
        handles.push((*key, handle));
    }
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
    for (key, handle) in handles {
        let tex = *renderer.resources().textures.get_ref(handle);
        renderer.resources().register_combined(key, tex.handle, tex.view, tex.dim, sampler);
    }

    let bgr = pso.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let mesh = StaticMesh {
        material_id: "pbr".into(),
        vertices: quad_vertices(),
        indices: Some(quad_indices()),
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh, None, "pbr".into());

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
    fn pbr_texture_loading() {
        run();
    }
}
