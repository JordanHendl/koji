use koji::material::*;
use koji::renderer::*;
use koji::utils::*;
use dashi::*;
use glam::*;
use serial_test::serial;
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
    let vert: &[u32] = include_spirv!("src/material/pbr.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/material/pbr.frag", frag, glsl);
    PipelineBuilder::new(ctx, "pbr_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(rp, subpass)
        .depth_enable(true)
        .cull_mode(CullMode::Back)
        .build()
}

#[test]
#[serial]
#[ignore]
fn bindless_lighting_effects() {
    let device = DeviceSelector::new().unwrap().select(DeviceFilter::default().add_required_type(DeviceType::Dedicated)).unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"lighting", &mut ctx).expect("renderer");

    let mut pso = build_pbr_pipeline(&mut ctx, renderer.render_pass(),0);
    let bgr = pso.create_bind_groups(&renderer.resources());
    renderer.register_pso(RenderStage::Opaque, pso, bgr);

    // Add some lights
    renderer.add_light(LightDesc{ position:[0.0,0.0,1.0], intensity:1.0, color:[1.0,1.0,1.0], range:10.0, direction:[0.0,-1.0,0.0], _pad:0 });
    renderer.add_light(LightDesc{ position:[1.0,1.0,1.0], intensity:0.5, color:[1.0,0.0,0.0], range:10.0, direction:[0.0,-1.0,0.0], _pad:0 });

    let mesh = StaticMesh {
        vertices: quad_vertices(),
        indices: Some(quad_indices()),
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh,None);

    renderer.present_frame().unwrap();
    ctx.destroy();
}

