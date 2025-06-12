#[cfg(feature = "gpu_tests")]
mod bindless_rendering {
    include!("../../tests/bindless_rendering.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    bindless_rendering::run();
}
