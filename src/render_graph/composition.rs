use super::{GraphNode, ResourceDesc};
use dashi::*;

/// Describes how multiple inputs are composited.
#[derive(Clone, Copy, Debug)]
pub enum BlendMode {
    /// Basic alpha blending of inputs in order.
    Alpha,
}

/// A graph node that composites multiple images into the swapchain image.
pub struct CompositionNode {
    name: String,
    /// Input images to composite.
    inputs: Vec<ResourceDesc>,
    /// Description of how inputs should be blended.
    mode: BlendMode,
    /// Format of the swapchain image.
    output: ResourceDesc,
}

impl CompositionNode {
    /// Create a new composition node using the given inputs and swapchain format.
    pub fn new(inputs: Vec<ResourceDesc>, swapchain_format: Format) -> Self {
        Self {
            name: "composition".to_string(),
            inputs,
            mode: BlendMode::Alpha,
            output: ResourceDesc { name: "swapchain".into(), format: swapchain_format },
        }
    }

    /// Set the blend mode for compositing.
    #[allow(dead_code)]
    pub fn blend_mode(mut self, mode: BlendMode) -> Self {
        self.mode = mode;
        self
    }
}

impl GraphNode for CompositionNode {
    fn name(&self) -> &str { &self.name }
    fn inputs(&self) -> Vec<ResourceDesc> { self.inputs.clone() }
    fn outputs(&self) -> Vec<ResourceDesc> { vec![self.output.clone()] }
    fn execute(&mut self, _ctx: &mut Context) -> Result<(), GPUError> {
        // In a full implementation, this would record a full-screen draw
        // that blends all inputs into the swapchain image according to `mode`.
        Ok(())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
