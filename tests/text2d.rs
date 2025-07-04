#![cfg(feature = "gpu_tests")]

use koji::material::pipeline_builder::PipelineBuilder;
use koji::renderer::*;
use koji::text::*;
use dashi::*;
use inline_spirv::include_spirv;

fn load_system_font() -> Vec<u8> {
    #[cfg(target_os = "windows")]
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/segoeui.ttf",
    ];
    #[cfg(target_os = "linux")]
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
    ];
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            return bytes;
        }
    }
    panic!("Could not locate a system font");
}

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

    let font_bytes = load_system_font();
    renderer.fonts_mut().register_font("default", &font_bytes);
    let text = TextRenderer2D::new(renderer.fonts(), "default");
    let info = StaticTextCreateInfo { text: "Hello", scale: 32.0, pos: [-0.5, 0.5], key: "glyph_tex" };
    let mesh = StaticText::new(&mut ctx, renderer.resources(), &text, info).unwrap();
    renderer.register_text_mesh(mesh);

    let vert_spv = make_vert();
    let frag_spv = make_frag();
    let mut pso = PipelineBuilder::new(&mut ctx, "text_pso")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass(renderer.render_pass(), 0)
        .build();
    let bgr = pso.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_pso(RenderStage::Text, pso, bgr);

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
