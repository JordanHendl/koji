use super::{GraphNode, RenderGraph};
use crate::canvas::Canvas;
use dashi::gpu::Format;

/// Convenience builder for assembling a [`RenderGraph`].
///
/// ```no_run
/// # use koji::render_graph::RenderGraphBuilder;
/// # use koji::canvas::CanvasBuilder;
/// # use dashi::gpu::{Context, Format};
/// # fn build_graph(ctx: &mut Context) -> Result<(), dashi::GPUError> {
/// let canvas = CanvasBuilder::new()
///     .extent([800, 600])
///     .color_attachment("color", Format::RGBA8)
///     .build(ctx)?;
/// let mut builder = RenderGraphBuilder::new();
/// builder.add_canvas(&canvas);
/// let graph = builder.build();
/// # Ok(()) }
/// ```
pub struct RenderGraphBuilder {
    graph: RenderGraph,
}

impl RenderGraphBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self {
            graph: RenderGraph::new(),
        }
    }

    /// Insert a generic [`GraphNode`] into the graph.
    pub fn add_node<N: GraphNode + 'static>(&mut self, node: N) -> &mut Self {
        self.graph.add_node(node);
        self
    }

    /// Add a [`Canvas`] to the graph.
    pub fn add_canvas(&mut self, canvas: &Canvas) -> &mut Self {
        self.graph.add_canvas(canvas);
        self
    }

    /// Register an external image resource by name and format.
    pub fn register_external_image(&mut self, name: &str, format: Format) -> &mut Self {
        self.graph.register_external_image(name, format);
        self
    }

    /// Connect two nodes by name.
    pub fn connect(&mut self, from: &str, to: &str) -> &mut Self {
        self.graph.connect(from, to);
        self
    }

    /// Finalize and return the constructed [`RenderGraph`].
    pub fn build(self) -> RenderGraph {
        self.graph
    }
}
