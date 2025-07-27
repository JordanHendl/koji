use dashi::*;
use koji::animation::clip::AnimationPlayer;
use koji::animation::Animator;
use koji::canvas::CanvasBuilder;
use koji::gltf::{load_scene, MeshData};
use koji::material::*;
use koji::render_graph::RenderGraph;
use koji::render_pass::RenderPassBuilder;
use koji::renderer::*;

#[cfg(feature = "gpu_tests")]
pub fn run(ctx: &mut Context) {
    let builder = RenderPassBuilder::new()
        .debug_name("MainPass")
        .viewport(Viewport {
            area: FRect2D { w: 320.0, h: 240.0, ..Default::default() },
            scissor: Rect2D { w: 320, h: 240, ..Default::default() },
            ..Default::default()
        })
        .color_attachment("color", Format::RGBA8)
        .subpass("main", ["color"], &[] as &[&str]);

    let mut renderer = Renderer::with_render_pass(320, 240, ctx, builder).unwrap();

    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(ctx)
        .unwrap();
    renderer.add_canvas(canvas.clone());

    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh {
        MeshData::Skeletal(m) => m.clone(),
        _ => panic!("expected skel"),
    };
    let clip = scene.animations[0].clone();
    let player = AnimationPlayer::new(clip);
    let animator = Animator::new(mesh.skeleton.clone());
    let instance = SkeletalInstance::with_player(ctx, animator, player).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let mut pso = build_skinning_pipeline(ctx, graph.output("color"));
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    renderer.play_animation(0, 0, 0.5);
    renderer.present_frame().unwrap();
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    {
        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = Context::new(&ContextInfo { device }).unwrap();
        run(&mut ctx);
        ctx.destroy();
    }
}
