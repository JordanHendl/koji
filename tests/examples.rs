use serial_test::serial;

mod sample {
    include!("../examples/sample/bin.rs");
}
mod pbr_spheres {
    include!("../examples/pbr_spheres/bin.rs");
}

#[test]
#[serial]
#[ignore]
fn run_sample() {
    sample::main();
}

#[test]
#[serial]
#[ignore]
fn run_pbr_spheres() {
    pbr_spheres::main();
}
