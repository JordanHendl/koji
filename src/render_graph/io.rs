use super::{RenderGraph, SerializableRenderGraph};

pub fn to_yaml(graph: &RenderGraph) -> Result<String, serde_yaml::Error> {
    let desc = SerializableRenderGraph::from(graph);
    serde_yaml::to_string(&desc)
}

pub fn from_yaml(data: &str) -> Result<RenderGraph, serde_yaml::Error> {
    let desc: SerializableRenderGraph = serde_yaml::from_str(data)?;
    Ok(RenderGraph::from(desc))
}

pub fn to_json(graph: &RenderGraph) -> Result<String, serde_json::Error> {
    let desc = SerializableRenderGraph::from(graph);
    serde_json::to_string_pretty(&desc)
}

pub fn from_json(data: &str) -> Result<RenderGraph, serde_json::Error> {
    let desc: SerializableRenderGraph = serde_json::from_str(data)?;
    Ok(RenderGraph::from(desc))
}
