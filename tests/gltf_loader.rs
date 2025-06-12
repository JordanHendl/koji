use koji::gltf::{load_scene, MeshData};

const TRIANGLE: &str = "tests/data/simple_triangle.gltf";
const SKIN: &str = "tests/data/simple_skin.gltf";

#[test]
fn load_triangle() {
    let scene = load_scene(TRIANGLE).expect("load");
    assert_eq!(scene.meshes.len(), 1);
    assert!(matches!(scene.meshes[0].mesh, MeshData::Static(_)));
}

#[test]
fn load_simple_skin() {
    let scene = load_scene(SKIN).expect("load");
    assert_eq!(scene.meshes.len(), 1);
    match &scene.meshes[0].mesh {
        MeshData::Skeletal(mesh) => {
            assert_eq!(mesh.skeleton.bones.len(), 2);
            assert_eq!(mesh.skeleton.bones[1].parent, Some(0));
        }
        _ => panic!("expected skeletal mesh"),
    }
    assert_eq!(scene.animations.len(), 1);
    let clip = &scene.animations[0];
    assert_eq!(clip.tracks.len(), 3);
    assert!(!clip.tracks[2].is_empty());
}

#[test]
fn invalid_path_errors() {
    assert!(load_scene("tests/data/does_not_exist.gltf").is_err());
}
