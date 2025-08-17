use super::{RenderGraph, SerializableRenderGraph};
use dashi::gpu::Context;

pub fn to_yaml(graph: &RenderGraph) -> Result<String, serde_yaml::Error> {
    let desc = SerializableRenderGraph::from(graph);
    serde_yaml::to_string(&desc)
}

pub fn from_yaml(ctx: &mut Context, data: &str) -> Result<RenderGraph, String> {
    let desc: SerializableRenderGraph =
        serde_yaml::from_str(data).map_err(|e| e.to_string())?;
    RenderGraph::from_desc(desc, ctx).map_err(|e| format!("{:?}", e))
}

pub fn to_json(graph: &RenderGraph) -> Result<String, serde_json::Error> {
    let desc = SerializableRenderGraph::from(graph);
    serde_json::to_string_pretty(&desc)
}

pub fn from_json(ctx: &mut Context, data: &str) -> Result<RenderGraph, String> {
    let desc: SerializableRenderGraph =
        serde_json::from_str(data).map_err(|e| e.to_string())?;
    RenderGraph::from_desc(desc, ctx).map_err(|e| format!("{:?}", e))
}
