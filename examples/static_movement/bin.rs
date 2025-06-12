#[cfg(feature = "gpu_tests")]
mod static_movement {
    include!("../../tests/static_movement.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    static_movement::run();
}
