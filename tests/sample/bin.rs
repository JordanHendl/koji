mod bin {
    include!("../../examples/sample/bin.rs");
}


pub use bin::render_sample_model;

pub fn main() {
    bin::main();
}
