mod bin {
    include!("../../examples/sample_deferred/bin.rs");
}


pub use bin::run;

pub fn main() {
    bin::main();
}
