use koji::material::*;
use koji::renderer::*;
use koji::animation::*;
use dashi::*;
use glam::*;
use serial_test::serial;
use inline_spirv::include_spirv;
use koji::material::pipeline_builder::PipelineBuilder;

use dashi::utils::Handle;
fn make_vertex(pos: [f32;3], uv:[f32;2]) -> SkeletalVertex {
    SkeletalVertex {
        position: pos,
        normal: [0.0,0.0,1.0],
        tangent: [1.0,0.0,0.0,1.0],
        uv,
        color: [1.0,1.0,1.0,1.0],
        joint_indices: [0,0,0,0],
        joint_weights: [1.0,0.0,0.0,0.0],
    }
}

fn quad_vertices() -> Vec<SkeletalVertex> {
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
fn skinned_mesh_rendering() {
    let device = DeviceSelector::new().unwrap().select(DeviceFilter::default().add_required_type(DeviceType::Dedicated)).unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"skinned", &mut ctx).expect("renderer");

    let mut pso = build_pbr_pipeline(&mut ctx, renderer.render_pass(),0);
    let bgr = pso.create_bind_groups(&renderer.resources());
    renderer.register_pso(RenderStage::Opaque, pso, bgr);

    let bones = vec![
        Bone { name:"root".into(), parent: None, inverse_bind: Mat4::IDENTITY },
        Bone { name:"child".into(), parent: Some(0), inverse_bind: Mat4::IDENTITY },
    ];
    let skeleton = Skeleton { bones };

    let mesh = SkeletalMesh {
        vertices: quad_vertices(),
        indices: Some(quad_indices()),
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
        skeleton,
        bone_buffer: None,
    };
    renderer.register_skeletal_mesh(mesh);

    renderer.update_skeletal_bones(0, &[Mat4::IDENTITY, Mat4::IDENTITY]);

    renderer.present_frame().unwrap();
    ctx.destroy();
}

