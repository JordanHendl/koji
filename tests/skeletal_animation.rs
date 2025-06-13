use koji::renderer::*;
use koji::gltf::{load_scene, MeshData};
use koji::material::*;
use koji::animation::{Animator};
use koji::animation::clip::AnimationPlayer;
use dashi::*;

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"anim", &mut ctx).unwrap();

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh { MeshData::Skeletal(m) => m.clone(), _ => panic!("expected skel") };
    let clip = scene.animations[0].clone();
    let player = AnimationPlayer::new(clip);
    let animator = Animator::new(mesh.skeleton.clone());
    let instance = SkeletalInstance::with_player(&mut ctx, animator, player).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let mut pso = build_skinning_pipeline(&mut ctx, renderer.render_pass(),0);
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
