use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::is_cyclic_directed;
use petgraph::visit::{Topo, EdgeRef};
use dashi::utils::*;
use dashi::*;

use dashi::gpu::RenderPass;

mod composition;
pub use composition::*;

#[derive(Clone, Debug)]
pub struct ResourceDesc {
    pub name: String,
    pub format: Format,
}

pub trait GraphNode {
    fn name(&self) -> &str;
    fn inputs(&self) -> Vec<ResourceDesc>;
    fn outputs(&self) -> Vec<ResourceDesc>;
    fn execute(&mut self, ctx: &mut Context) -> Result<(), GPUError>;
}

pub struct RenderPassNode {
    name: String,
    pass: Handle<RenderPass>,
    inputs: Vec<ResourceDesc>,
    outputs: Vec<ResourceDesc>,
}

impl RenderPassNode {
    pub fn new(name: impl Into<String>, pass: Handle<RenderPass>, inputs: Vec<ResourceDesc>, outputs: Vec<ResourceDesc>) -> Self {
        Self { name: name.into(), pass, inputs, outputs }
    }

    pub fn geometry(pass: Handle<RenderPass>) -> Self {
        Self::new("geometry", pass, Vec::new(), Vec::new())
    }

    pub fn postprocess(pass: Handle<RenderPass>) -> Self {
        Self::new("postprocess", pass, Vec::new(), Vec::new())
    }
}

impl GraphNode for RenderPassNode {
    fn name(&self) -> &str { &self.name }
    fn inputs(&self) -> Vec<ResourceDesc> { self.inputs.clone() }
    fn outputs(&self) -> Vec<ResourceDesc> { self.outputs.clone() }
    fn execute(&mut self, _ctx: &mut Context) -> Result<(), GPUError> {
        // In a full implementation this would record commands for the render pass
        let _ = self.pass; // silence unused field
        Ok(())
    }
}

pub struct ExternalImageNode {
    name: String,
    format: Format,
}

impl ExternalImageNode {
    pub fn new(name: impl Into<String>, format: Format) -> Self {
        Self { name: name.into(), format }
    }
}

impl GraphNode for ExternalImageNode {
    fn name(&self) -> &str { &self.name }
    fn inputs(&self) -> Vec<ResourceDesc> { Vec::new() }
    fn outputs(&self) -> Vec<ResourceDesc> {
        vec![ResourceDesc { name: self.name.clone(), format: self.format }]
    }
    fn execute(&mut self, _ctx: &mut Context) -> Result<(), GPUError> { Ok(()) }
}

pub struct RenderGraph {
    graph: DiGraph<Box<dyn GraphNode>, ()>,
    indices: HashMap<String, NodeIndex>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self { graph: DiGraph::new(), indices: HashMap::new() }
    }

    pub fn add_node<N: GraphNode + 'static>(&mut self, node: N) {
        let name = node.name().to_string();
        let idx = self.graph.add_node(Box::new(node));
        self.indices.insert(name, idx);
    }

    pub fn connect(&mut self, from: &str, to: &str) {
        if let (Some(&a), Some(&b)) = (self.indices.get(from), self.indices.get(to)) {
            self.graph.add_edge(a, b, ());
        }
    }

    pub fn register_external_image(&mut self, name: &str, format: Format) {
        self.add_node(ExternalImageNode::new(name, format));
    }

    pub fn validate(&self) -> Result<(), String> {
        if is_cyclic_directed(&self.graph) {
            return Err("Render graph contains cycles".to_string());
        }
        for edge in self.graph.edge_references() {
            let src = edge.source();
            let dst = edge.target();
            let src_node = &self.graph[src];
            let dst_node = &self.graph[dst];
            for out in src_node.outputs() {
                for inp in dst_node.inputs() {
                    if out.name == inp.name && out.format != inp.format {
                        return Err(format!("Format mismatch for resource {}", out.name));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn execute(&mut self, ctx: &mut Context) -> Result<(), GPUError> {
        self.validate().map_err(|_| GPUError::LibraryError())?;
        let mut topo = Topo::new(&self.graph);
        while let Some(idx) = topo.next(&self.graph) {
            let node = self.graph.node_weight_mut(idx).unwrap();
            node.execute(ctx)?;
        }
        Ok(())
    }
}

