#[cfg(feature = "gpu_tests")]
mod pbr_renderer {
    include!("../../tests/pbr_renderer.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    pbr_renderer::run();
}
