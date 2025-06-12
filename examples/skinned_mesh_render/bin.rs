#[cfg(feature = "gpu_tests")]
mod skinned_mesh_render {
    include!("../../tests/skinned_mesh_render.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    skinned_mesh_render::run();
}
