use koji::render_graph::{RenderGraph, ResourceDesc, CompositionNode};
use dashi::gpu;
use dashi::*;
use serial_test::serial;

fn setup_ctx() -> gpu::Context {
    gpu::Context::headless(&Default::default()).unwrap()
}

#[test]
#[serial]
fn composition_node_registers_inputs() {
    let node = CompositionNode::new(
        vec![ResourceDesc { name: "a".into(), format: Format::RGBA8 }],
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
        vec![ResourceDesc { name: "input".into(), format: Format::RGBA8 }],
        Format::BGRA8,
    );
    graph.add_node(node);
    graph.connect("input", "composition");
    graph.execute(&mut ctx).unwrap();
    ctx.destroy();
}
