#[path = "../../test/sample_deferred/bin.rs"]
mod bin;


pub use bin::run;

pub fn main() {
    bin::main();
}
