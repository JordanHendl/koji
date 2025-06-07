#[path = "../../test/sample_shadows/bin.rs"]
mod bin;

pub use bin::run;

pub fn main() {
    bin::main();
}
