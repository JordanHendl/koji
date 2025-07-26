use inline_spirv::include_spirv;
use dashi::*;
use crate::material::pipeline_builder::PipelineBuilder;

pub fn build_skinning_pipeline(
    ctx: &mut Context,
    target: crate::render_graph::GraphOutput,
) -> crate::material::PSO {
    let vert: &[u32] = include_spirv!("src/renderer/skinning.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/renderer/skinning.frag", frag, glsl);
    PipelineBuilder::new(ctx, "skinning_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(target)
        .build()
}
