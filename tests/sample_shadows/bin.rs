mod bin {
    include!("../../examples/sample_shadows/bin.rs");
}


pub use bin::run;

pub fn main() {
    bin::main();
}
