use koji::renderer::*;
use koji::gltf::{load_scene, MeshData};
use koji::material::*;
use glam::Mat4;
use koji::animation::Animator;
use dashi::*;
use serial_test::serial;

#[test]
#[serial]
#[ignore]
fn skinned_mesh_render() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"skin", &mut ctx).unwrap();

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh { MeshData::Skeletal(m) => m.clone(), _ => panic!("expected skel") };
    let bone_count = mesh.skeleton.bone_count();
    let instance = SkeletalInstance::new(&mut ctx, Animator::new(mesh.skeleton.clone())).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let mut pso = build_skinning_pipeline(&mut ctx, renderer.render_pass(),0);
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso,bgr);

    let mats = vec![Mat4::IDENTITY; bone_count];
    renderer.update_skeletal_bones(0,0,&mats);
    renderer.present_frame().unwrap();
    ctx.destroy();
}
