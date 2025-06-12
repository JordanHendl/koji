use koji::material::pipeline_builder::PipelineBuilder;
use koji::renderer::*;
use koji::text::*;
use dashi::*;
use inline_spirv::include_spirv;

fn make_vert() -> Vec<u32> {
    include_spirv!("assets/shaders/text.vert", vert).to_vec()
}

fn make_frag() -> Vec<u32> {
    include_spirv!("assets/shaders/text.frag", frag).to_vec()
}

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap().select(DeviceFilter::default().add_required_type(DeviceType::Dedicated)).unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
    let mut renderer = Renderer::new(320, 240, "text", &mut ctx).expect("renderer");

    let vert_spv = make_vert();
    let frag_spv = make_frag();
    let mut pso = PipelineBuilder::new(&mut ctx, "text_pso")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass(renderer.render_pass(), 0)
        .build();
    let bgr = pso.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let font_bytes: &[u8] = include_bytes!("../assets/data/DejaVuSans.ttf");
    let text = TextRenderer2D::new(font_bytes);
    let dim = text.upload_text_texture(&mut ctx, renderer.resources(), "glyph_tex", "Hello", 32.0);
    let mesh = text.make_quad(dim, [-0.5, 0.5]);
    renderer.register_text_mesh(mesh);

    renderer.present_frame().unwrap();
    ctx.destroy();
}

#[cfg(all(test, feature = "gpu_tests"))]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    #[ignore]
    fn draw_text_2d() {
        run();
    }
}
