use dashi::utils::*;
use dashi::*;
use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, Topo};
use std::collections::HashMap;

use dashi::gpu::RenderPass;
use serde::{Deserialize, Serialize};

mod composition;
pub use composition::*;
pub mod io;
pub use io::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceDesc {
    pub name: String,
    pub format: Format,
}

pub trait GraphNode {
    fn name(&self) -> &str;
    fn inputs(&self) -> Vec<ResourceDesc>;
    fn outputs(&self) -> Vec<ResourceDesc>;
    fn execute(&mut self, ctx: &mut Context) -> Result<(), GPUError>;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct RenderPassNode {
    name: String,
    pass: Handle<RenderPass>,
    inputs: Vec<ResourceDesc>,
    outputs: Vec<ResourceDesc>,
}

impl RenderPassNode {
    pub fn new(
        name: impl Into<String>,
        pass: Handle<RenderPass>,
        inputs: Vec<ResourceDesc>,
        outputs: Vec<ResourceDesc>,
    ) -> Self {
        Self {
            name: name.into(),
            pass,
            inputs,
            outputs,
        }
    }

    pub fn geometry(pass: Handle<RenderPass>) -> Self {
        Self::new("geometry", pass, Vec::new(), Vec::new())
    }

    pub fn postprocess(pass: Handle<RenderPass>) -> Self {
        Self::new("postprocess", pass, Vec::new(), Vec::new())
    }
}

impl GraphNode for RenderPassNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<ResourceDesc> {
        self.inputs.clone()
    }
    fn outputs(&self) -> Vec<ResourceDesc> {
        self.outputs.clone()
    }
    fn execute(&mut self, _ctx: &mut Context) -> Result<(), GPUError> {
        // In a full implementation this would record commands for the render pass
        let _ = self.pass; // silence unused field
        Ok(())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ExternalImageNode {
    name: String,
    format: Format,
}

impl ExternalImageNode {
    pub fn new(name: impl Into<String>, format: Format) -> Self {
        Self {
            name: name.into(),
            format,
        }
    }
}

impl GraphNode for ExternalImageNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<ResourceDesc> {
        Vec::new()
    }
    fn outputs(&self) -> Vec<ResourceDesc> {
        vec![ResourceDesc {
            name: self.name.clone(),
            format: self.format,
        }]
    }
    fn execute(&mut self, _ctx: &mut Context) -> Result<(), GPUError> {
        Ok(())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct RenderGraph {
    graph: DiGraph<Box<dyn GraphNode>, ()>,
    indices: HashMap<String, NodeIndex>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNodeDesc {
    pub name: String,
    pub inputs: Vec<ResourceDesc>,
    pub outputs: Vec<ResourceDesc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableRenderGraph {
    pub nodes: Vec<GraphNodeDesc>,
    pub edges: Vec<(String, String)>,
}

pub struct SimpleNode {
    name: String,
    inputs: Vec<ResourceDesc>,
    outputs: Vec<ResourceDesc>,
}

impl From<GraphNodeDesc> for SimpleNode {
    fn from(desc: GraphNodeDesc) -> Self {
        Self {
            name: desc.name,
            inputs: desc.inputs,
            outputs: desc.outputs,
        }
    }
}

impl GraphNode for SimpleNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<ResourceDesc> {
        self.inputs.clone()
    }
    fn outputs(&self) -> Vec<ResourceDesc> {
        self.outputs.clone()
    }
    fn execute(&mut self, _ctx: &mut Context) -> Result<(), GPUError> {
        Ok(())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            indices: HashMap::new(),
        }
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

    pub fn render_pass_for_output(&self, output: &str) -> Option<(Handle<RenderPass>, Format)> {
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            for out in node.outputs() {
                if out.name == output {
                    if let Some(rp_node) = node.as_any().downcast_ref::<RenderPassNode>() {
                        return Some((rp_node.pass, out.format));
                    } else {
                        return None;
                    }
                }
            }
        }
        None
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

    pub fn node_names(&self) -> Vec<String> {
        self.graph
            .node_indices()
            .map(|i| self.graph[i].name().to_string())
            .collect()
    }

    pub fn edges(&self) -> Vec<(String, String)> {
        self.graph
            .edge_references()
            .map(|e| {
                let a = self.graph[e.source()].name().to_string();
                let b = self.graph[e.target()].name().to_string();
                (a, b)
            })
            .collect()
    }

    pub fn output_images(&self) -> Vec<String> {
        self.graph
            .node_indices()
            .flat_map(|i| {
                self.graph[i]
                    .outputs()
                    .into_iter()
                    .map(|r| r.name)
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

impl From<&RenderGraph> for SerializableRenderGraph {
    fn from(g: &RenderGraph) -> Self {
        let nodes = g
            .graph
            .node_indices()
            .map(|i| {
                let node = &g.graph[i];
                GraphNodeDesc {
                    name: node.name().to_string(),
                    inputs: node.inputs(),
                    outputs: node.outputs(),
                }
            })
            .collect();
        let edges = g
            .graph
            .edge_references()
            .map(|e| {
                let a = g.graph[e.source()].name().to_string();
                let b = g.graph[e.target()].name().to_string();
                (a, b)
            })
            .collect();
        Self { nodes, edges }
    }
}

impl From<SerializableRenderGraph> for RenderGraph {
    fn from(desc: SerializableRenderGraph) -> Self {
        let mut g = RenderGraph::new();
        for n in desc.nodes {
            g.add_node(SimpleNode::from(n));
        }
        for (a, b) in desc.edges {
            g.connect(&a, &b);
        }
        g
    }
}
