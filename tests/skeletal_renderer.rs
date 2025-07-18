use koji::renderer::*;
use koji::gltf::{load_scene, MeshData};
use koji::material::*;
use glam::Mat4;
use koji::animation::Animator;
use dashi::*;

#[cfg(feature = "gpu_tests")]
pub fn run_simple_skeleton() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
    let mut renderer = Renderer::new(320, 240, "skin", &mut ctx).unwrap();

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh {
        MeshData::Skeletal(m) => m.clone(),
        _ => panic!("expected skeletal mesh"),
    };
    let bone_count = mesh.skeleton.bone_count();
    let instance = SkeletalInstance::new(&mut ctx, Animator::new(mesh.skeleton.clone())).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let mut pso = build_skinning_pipeline(&mut ctx, renderer.render_pass(), 0);
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    let mats = vec![Mat4::IDENTITY; bone_count];
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.present_frame().unwrap();
    ctx.destroy();
}

#[cfg(feature = "gpu_tests")]
pub fn run_update_bones_twice() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
    let mut renderer = Renderer::new(320, 240, "skin", &mut ctx).unwrap();

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh {
        MeshData::Skeletal(m) => m.clone(),
        _ => panic!("expected skeletal mesh"),
    };
    let bone_count = mesh.skeleton.bone_count();
    let instance = SkeletalInstance::new(&mut ctx, Animator::new(mesh.skeleton.clone())).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let mut pso = build_skinning_pipeline(&mut ctx, renderer.render_pass(), 0);
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    let mats = vec![Mat4::IDENTITY; bone_count];
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.update_skeletal_bones(0, 0, &mats);
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
    fn render_simple_skeleton() {
        run_simple_skeleton();
    }

    #[test]
    #[serial]
    #[ignore]
    fn update_bones_twice() {
        run_update_bones_twice();
    }
}
