#[cfg(feature = "gpu_tests")]
mod skeletal_renderer {
    include!("../../tests/skeletal_renderer.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    {
        skeletal_renderer::run_simple_skeleton();
        skeletal_renderer::run_update_bones_twice();
    }
}
