use crate::material::pipeline_builder::PipelineBuilder;
use crate::material::PSO;
use dashi::{gpu::RenderPass, Context, utils::Handle};

/// Build a simple pipeline for rendering [`SkeletalMesh`] objects.
///
/// The vertex shader performs linear blend skinning and outputs vertex color.
/// A minimal fragment shader simply passes this color through.
pub fn build_skinning_pipeline(
    ctx: &mut Context,
    rp: Handle<RenderPass>,
    subpass: u32,
) -> PSO {
    let vert: &[u32] = inline_spirv::include_spirv!(
        "src/renderer/skinning.vert",
        vert,
        glsl
    );
    let frag: &[u32] = inline_spirv::include_spirv!(
        "shaders/test_triangle.frag",
        frag,
        glsl
    );
    PipelineBuilder::new(ctx, "skinning_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(rp, subpass)
        .build()
}

