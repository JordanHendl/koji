use koji::renderer::*;
use koji::gltf::{load_scene, MeshData};
use koji::material::*;
use koji::animation::Animator;
use koji::animation::clip::AnimationPlayer;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use koji::render_pass::RenderPassBuilder;
use inline_spirv::include_spirv;
use dashi::*;

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
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

    let mut renderer = Renderer::with_render_pass(320, 240, &mut ctx, builder).unwrap();
    renderer.add_canvas(canvas);

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh { MeshData::Skeletal(m) => m.clone(), _ => panic!("expected skel") };
    let clip = scene.animations[0].clone();
    let player = AnimationPlayer::new(clip);
    let animator = Animator::new(mesh.skeleton.clone());
    let instance = SkeletalInstance::with_player(&mut ctx, animator, player).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let vert: &[u32] = include_spirv!("src/renderer/skinning.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/renderer/skinning.frag", frag, glsl);
    let mut pso = PipelineBuilder::new(&mut ctx, "skinning_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(renderer.graph().output("color"))
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso,bgr);

    renderer.play_animation(0,0,0.5);
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
    fn skeletal_animation() {
        run();
    }
}
