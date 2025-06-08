#[path = "../examples/sample/bin.rs"]
mod sample;
#[path = "../examples/sample_deferred/bin.rs"]
mod sample_deferred;
#[path = "../examples/sample_shadows/bin.rs"]
mod sample_shadows;

#[test]
#[ignore]
fn run_sample() {
    sample::main();
}

#[test]
#[ignore]
fn run_deferred_sample() {
    sample_deferred::main();
}

#[test]
#[ignore]
fn run_shadow_sample() {
    sample_shadows::main();
}
