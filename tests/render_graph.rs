#![cfg(feature = "gpu_tests")]

use dashi::gpu;
use dashi::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::{CompositionNode, GraphNode, RenderGraph, ResourceDesc};
use serial_test::serial;

fn setup_ctx() -> gpu::Context {
    gpu::Context::headless(&Default::default()).unwrap()
}

#[test]
#[serial]
fn composition_node_registers_inputs() {
    let node = CompositionNode::new(
        vec![ResourceDesc {
            name: "a".into(),
            format: Format::RGBA8,
        }],
        Format::BGRA8,
    );
    assert_eq!(node.inputs().len(), 1);
    assert_eq!(node.outputs()[0].name, "swapchain");
}

#[test]
#[serial]
fn render_graph_executes_with_composition() {
    let mut ctx = setup_ctx();
    let mut graph = RenderGraph::new();
    graph.register_external_image("input", Format::RGBA8);
    let node = CompositionNode::new(
        vec![ResourceDesc {
            name: "input".into(),
            format: Format::RGBA8,
        }],
        Format::BGRA8,
    );
    graph.add_node(node);
    graph.connect("input", "composition");
    graph.execute(&mut ctx).unwrap();
    ctx.destroy();
}

#[test]
fn graph_yaml_roundtrip() {
    let mut graph = RenderGraph::new();
    graph.register_external_image("img", Format::RGBA8);
    let yaml = koji::render_graph::to_yaml(&graph).unwrap();
    let loaded = koji::render_graph::from_yaml(&yaml).unwrap();
    assert_eq!(graph.node_names(), loaded.node_names());
}

#[test]
#[serial]
fn canvas_node_outputs() {
    let mut ctx = setup_ctx();
    let canvas = CanvasBuilder::new()
        .extent([1, 1])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();

    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);
    assert_eq!(graph.output_images(), vec!["color".to_string()]);
    let rp = graph.render_pass_for_output("color");
    assert!(matches!(rp, Some((_p, Format::RGBA8))));
    ctx.destroy();
}
