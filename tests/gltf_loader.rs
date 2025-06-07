use koji::gltf::{load_scene, MeshData};

#[test]
fn load_triangle() {
    let scene = load_scene("tests/data/simple_triangle.gltf").expect("load");
    assert_eq!(scene.meshes.len(), 1);
}

#[test]
fn load_simple_skin() {
    let scene = load_scene("tests/data/simple_skin.gltf").expect("load");
    assert_eq!(scene.meshes.len(), 1);
    match &scene.meshes[0].mesh {
        MeshData::Skeletal(mesh) => {
            assert_eq!(mesh.skeleton.bones.len(), 2);
            assert_eq!(mesh.skeleton.bones[1].parent, Some(0));
        }
        _ => panic!("expected skeletal mesh"),
    }
}
