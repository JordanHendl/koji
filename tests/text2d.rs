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

fn expected_dims(text: &str, scale: f32, font_bytes: &[u8]) -> [u32; 2] {
    use rusttype::{Font, Scale, point};
    let font = Font::try_from_bytes(font_bytes).expect("font");
    let scale = Scale::uniform(scale);
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font
        .layout(text, scale, point(0.0, v_metrics.ascent))
        .collect();
    let width = glyphs
        .iter()
        .rev()
        .filter_map(|g| g.pixel_bounding_box().map(|bb| bb.max.x as i32))
        .next()
        .unwrap_or(0);
    let height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
    [width as u32, height]
}

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap().select(DeviceFilter::default().add_required_type(DeviceType::Dedicated)).unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
    let mut renderer = Renderer::new(320, 240, "text", &mut ctx).expect("renderer");

    let font_bytes = load_system_font();
    renderer.fonts_mut().register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(renderer.fonts(), "default");
    let info = StaticTextCreateInfo { text: "Hello", scale: 32.0, pos: [-0.5, 0.5], key: "glyph_tex", screen_size: [320.0, 240.0], color: [1.0; 4], bold: false, italic: false };
    let mesh = StaticText::new(&mut ctx, renderer.resources(), &mut text, info).unwrap();
    renderer.register_text_mesh(mesh);
    text.register_textures(renderer.resources());

    let vert_spv = make_vert();
    let frag_spv = make_frag();
    let mut pso = PipelineBuilder::new(&mut ctx, "text_pso")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass((renderer.render_pass(), 0))
        .build_with_resources(renderer.resources())
        .unwrap();
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
