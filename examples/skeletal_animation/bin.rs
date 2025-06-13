#[cfg(feature = "gpu_tests")]
mod skeletal_animation {
    include!("../../tests/skeletal_animation.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    skeletal_animation::run();
}
