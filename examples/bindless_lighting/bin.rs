#[cfg(feature = "gpu_tests")]
mod bindless_lighting {
    include!("../../tests/bindless_lighting.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    bindless_lighting::run();
}
