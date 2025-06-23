use glam::Mat4;
use koji::animation::Animator;
use koji::gltf::{load_scene, MeshData};
use koji::material::*;
use koji::renderer::*;
use dashi::*;

#[cfg(feature = "gpu_tests")]
fn run_simple_skeleton(ctx: &mut Context) {
    let mut renderer = Renderer::new(320, 240, "skin", ctx).unwrap();

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh {
        MeshData::Skeletal(m) => m.clone(),
        _ => panic!("expected skeletal mesh"),
    };
    let bone_count = mesh.skeleton.bone_count();
    let instance = SkeletalInstance::new(ctx, Animator::new(mesh.skeleton.clone())).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let mut pso = build_skinning_pipeline(ctx, renderer.render_pass(), 0);
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    let mats = vec![Mat4::IDENTITY; bone_count];
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.present_frame().unwrap();
}

#[cfg(feature = "gpu_tests")]
fn run_update_bones_twice(ctx: &mut Context) {
    let mut renderer = Renderer::new(320, 240, "skin", ctx).unwrap();

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh {
        MeshData::Skeletal(m) => m.clone(),
        _ => panic!("expected skeletal mesh"),
    };
    let bone_count = mesh.skeleton.bone_count();
    let instance = SkeletalInstance::new(ctx, Animator::new(mesh.skeleton.clone())).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let mut pso = build_skinning_pipeline(ctx, renderer.render_pass(), 0);
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    let mats = vec![Mat4::IDENTITY; bone_count];
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.present_frame().unwrap();
}

#[cfg(feature = "gpu_tests")]
pub fn run(ctx: &mut Context) {
    run_simple_skeleton(ctx);
    run_update_bones_twice(ctx);
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
