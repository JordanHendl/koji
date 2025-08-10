#![cfg(feature = "gpu_tests")]

use dashi::gpu::{Context, ContextInfo};
use dashi::Format;
use koji::canvas::CanvasBuilder;
use koji::renderer::Renderer;
use koji::utils::diff_rgba8;
use serial_test::serial;

#[test]
#[serial]
fn headless_frame_difference() {
    // Initialize a headless GPU context and a simple canvas
    let mut ctx = Context::headless(&ContextInfo::default()).unwrap();
    let canvas = CanvasBuilder::new()
        .extent([8, 8])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut renderer = Renderer::with_canvas_headless(8, 8, &mut ctx, canvas).unwrap();

    // Render two identical frames
    renderer.present_frame().unwrap();
    let frame_a = renderer.read_color_target("color");
    renderer.present_frame().unwrap();
    let frame_b = renderer.read_color_target("color");
    assert_eq!(diff_rgba8(&frame_a, &frame_b), 0.0);

    // Change the clear color and render again
    renderer.set_clear_color([1.0, 0.0, 0.0, 1.0]);
    renderer.present_frame().unwrap();
    let frame_c = renderer.read_color_target("color");
    assert!(diff_rgba8(&frame_a, &frame_c) > 0.0);

    drop(renderer);
    ctx.destroy();
}
