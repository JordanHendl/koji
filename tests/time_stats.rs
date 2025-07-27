use koji::renderer::TimeStats;
use std::time::Duration;
use koji::renderer::Renderer;
use koji::utils::ResourceBinding;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use koji::render_pass::RenderPassBuilder;
use dashi::gpu;
use dashi::Format;
use serial_test::serial;

#[test]
fn update_tracks_elapsed_and_delta() {
    let mut stats = TimeStats::new();
    std::thread::sleep(Duration::from_millis(5));
    stats.update();
    assert!(stats.total_time > 0.0);
    let first_total = stats.total_time;
    let first_delta = stats.delta_time;
    assert!(first_delta > 0.0);
    std::thread::sleep(Duration::from_millis(5));
    stats.update();
    assert!(stats.total_time > first_total);
    assert!(stats.delta_time > 0.0);
    assert!(stats.delta_time <= stats.total_time);
}

#[test]
#[serial]
#[cfg_attr(not(feature = "gpu_tests"), ignore)]
#[ignore]
fn renderer_updates_time_buffer() {
    let device = gpu::DeviceSelector::new()
        .unwrap()
        .select(gpu::DeviceFilter::default().add_required_type(gpu::DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = gpu::Context::new(&gpu::ContextInfo { device }).unwrap();

    let canvas = CanvasBuilder::new()
        .extent([64, 64])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let builder = RenderPassBuilder::new()
        .debug_name("MainPass")
        .color_attachment("color", Format::RGBA8)
        .subpass("main", ["color"], &[] as &[&str]);

    let mut renderer = Renderer::with_render_pass(64, 64, &mut ctx, builder).unwrap();
    renderer.add_canvas(canvas);

    renderer.present_frame().unwrap();
    std::thread::sleep(Duration::from_millis(5));
    renderer.present_frame().unwrap();

    let handle = match renderer.resources().get("time") {
        Some(ResourceBinding::Uniform(h)) => *h,
        _ => panic!("expected time buffer"),
    };
    let data: [f32; 2] = {
        let slice = ctx.map_buffer::<u8>(handle).unwrap();
        let bytes = &slice[..std::mem::size_of::<[f32; 2]>()];
        let val = *bytemuck::from_bytes(bytes);
        ctx.unmap_buffer(handle).unwrap();
        val
    };

    assert!(data[0] > 0.0);
    assert!(data[1] > 0.0);
    ctx.destroy();
}
