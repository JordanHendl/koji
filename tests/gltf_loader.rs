use koji::gltf::load_scene;

#[test]
fn load_triangle() {
    let scene = load_scene("tests/data/simple_triangle.gltf").expect("load");
    assert_eq!(scene.meshes.len(), 1);
}
