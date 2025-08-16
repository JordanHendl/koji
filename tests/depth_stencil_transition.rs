#![cfg(feature = "gpu_tests")]

use serial_test::serial;
use koji::render_pass::RenderPassBuilder;
use dashi::gpu::{Context, ContextInfo};
use dashi::Format;
use ash::vk;

fn setup_ctx() -> Context {
    Context::headless(&ContextInfo::default()).unwrap()
}

#[test]
#[serial]
fn transition_depth_stencil_image() {
    let mut ctx = setup_ctx();

    let builder = RenderPassBuilder::new()
        .debug_name("DepthStencil")
        .extent([4, 4])
        .color_attachment("color", Format::RGBA8)
        .depth_attachment("depth", Format::D24S8)
        .subpass("main", ["color"], &[] as &[&str]);

    let (_rp, targets, _all) = builder.build_with_images(&mut ctx).unwrap();
    let depth_view = targets[0].depth.as_ref().unwrap().attachment.img;

    let mut cmd = ctx.begin_command_list(&Default::default()).unwrap();
    ctx.transition_image(cmd.cmd_buf, depth_view, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    ctx.transition_image(cmd.cmd_buf, depth_view, vk::ImageLayout::GENERAL);
    let fence = ctx.submit(&mut cmd, &Default::default()).unwrap();
    ctx.wait(fence).unwrap();

    ctx.destroy();
}

