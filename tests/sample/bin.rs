#[path = "../../test/sample/bin.rs"]
mod bin;


pub use bin::render_sample_model;

pub fn main() {
    bin::main();
}
