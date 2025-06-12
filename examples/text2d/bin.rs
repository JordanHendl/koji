#[cfg(feature = "gpu_tests")]
mod text2d {
    include!("../../tests/text2d.rs");
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    text2d::run();
}
