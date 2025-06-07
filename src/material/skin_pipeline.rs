use inline_spirv::include_spirv;
use dashi::{*, utils::Handle};
use crate::material::pipeline_builder::PipelineBuilder;

pub fn build_skinning_pipeline(ctx: &mut Context, rp: Handle<RenderPass>, subpass: u32) -> crate::material::PSO {
    let vert: &[u32] = include_spirv!("src/renderer/skinning.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/renderer/skinning.frag", frag, glsl);
    PipelineBuilder::new(ctx, "skinning_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(rp, subpass)
        .build()
}
