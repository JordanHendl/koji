mod bin {
    include!("../../examples/pbr_spheres/bin.rs");
}

pub use bin::run;

pub fn main() {
    bin::main();
}
