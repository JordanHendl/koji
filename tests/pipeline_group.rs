use koji::renderer::{Renderer, test_hooks, StaticMesh, Vertex, SkeletalMesh, SkeletalVertex, SkeletalInstance};
use koji::material::pipeline_builder::PipelineBuilder;
use koji::canvas::CanvasBuilder;
use koji::animation::{Skeleton, Bone, Animator};
use dashi::gpu::{Context, ContextInfo};
use dashi::{Format};
use inline_spirv::{inline_spirv, include_spirv};
use serial_test::serial;

fn make_ctx() -> Context {
    Context::headless(&ContextInfo::default()).unwrap()
}

fn simple_vert() -> Vec<u32> {
    inline_spirv!(r"#version 450
        layout(location=0) in vec3 pos;
        void main(){ gl_Position = vec4(pos,1.0); }", vert).to_vec()
}

fn simple_frag() -> Vec<u32> {
    inline_spirv!(r"#version 450
        layout(location=0) out vec4 o;
        void main(){ o = vec4(1.0); }", frag).to_vec()
}

fn simple_vertex(p: [f32;3]) -> Vertex {
    Vertex { position:p, normal:[0.0,0.0,1.0], tangent:[1.0,0.0,0.0,1.0], uv:[0.0,0.0], color:[1.0,1.0,1.0,1.0] }
}

fn simple_skel_vertex(p: [f32;3]) -> SkeletalVertex {
    SkeletalVertex { position:p, normal:[0.0,0.0,1.0], tangent:[1.0,0.0,0.0,1.0], uv:[0.0,0.0], color:[1.0,1.0,1.0,1.0], joint_indices:[0,0,0,0], joint_weights:[1.0,0.0,0.0,0.0] }
}

#[test]
#[serial]
#[cfg_attr(not(feature = "gpu_tests"), ignore)]
#[ignore]
fn static_pipeline_groups_once() {
    let mut ctx = make_ctx();
    let canvas = CanvasBuilder::new()
        .extent([64, 64])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut renderer = Renderer::with_canvas_headless(64, 64, &mut ctx, canvas).unwrap();

    let mut pso = PipelineBuilder::new(&mut ctx, "p")
        .vertex_shader(&simple_vert())
        .fragment_shader(&simple_frag())
        .render_pass(renderer.graph().output("color"))
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let mesh1 = StaticMesh {
        material_id: "p".into(),
        vertices: vec![simple_vertex([0.0,0.0,0.0])],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    let mesh2 = StaticMesh {
        material_id: "p".into(),
        vertices: vec![simple_vertex([0.0,0.0,0.0])],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_static_mesh(mesh1, None, "p".into());
    renderer.register_static_mesh(mesh2, None, "p".into());

    renderer.present_frame().unwrap();
    let events = test_hooks::take_draw_events();
    assert_eq!(events, vec!["begin_static", "end_static"]);
    ctx.destroy();
}

#[test]
#[serial]
#[cfg_attr(not(feature = "gpu_tests"), ignore)]
#[ignore]
fn skeletal_pipeline_groups_once() {
    let mut ctx = make_ctx();
    let canvas = CanvasBuilder::new()
        .extent([64, 64])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut renderer = Renderer::with_canvas_headless(64, 64, &mut ctx, canvas).unwrap();

    let vert: &[u32] = include_spirv!("src/renderer/skinning.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/renderer/skinning.frag", frag, glsl);
    let mut pso = PipelineBuilder::new(&mut ctx, "skel")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.graph().output("color"))
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    let skeleton = Skeleton { bones: vec![Bone::default()] };
    let mut mesh = SkeletalMesh {
        material_id: "skel".into(),
        vertices: vec![simple_skel_vertex([0.0,0.0,0.0])],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
        skeleton: skeleton.clone(),
        bone_buffer: None,
    };
    let inst1 = SkeletalInstance::new(&mut ctx, Animator::new(skeleton.clone())).unwrap();
    let inst2 = SkeletalInstance::new(&mut ctx, Animator::new(skeleton)).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![inst1, inst2], "skel".into());

    renderer.present_frame().unwrap();
    let events = test_hooks::take_draw_events();
    assert_eq!(events, vec!["begin_skeletal", "end_skeletal"]);
    ctx.destroy();
}
