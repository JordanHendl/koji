use serial_test::serial;

mod sample {
    include!("../examples/sample/bin.rs");
}
mod deferred_sample {
    include!("../examples/sample_deferred/bin.rs");
}
mod shadow_sample {
    include!("../examples/sample_shadows/bin.rs");
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
fn run_deferred_sample() {
    deferred_sample::main();
}

#[test]
#[serial]
#[ignore]
fn run_shadow_sample() {
    shadow_sample::main();
}

#[test]
#[serial]
#[ignore]
fn run_pbr_spheres() {
    pbr_spheres::main();
}
