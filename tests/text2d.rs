#![cfg(feature = "gpu_tests")]

use koji::material::pipeline_builder::PipelineBuilder;
use koji::renderer::*;
use koji::text::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use dashi::*;
use inline_spirv::include_spirv;

fn load_system_font() -> Result<Vec<u8>, String> {
    if let Ok(path) = std::env::var("KOJI_FONT_PATH") {
        return std::fs::read(&path)
            .map_err(|e| format!("Failed to read font at {}: {}", path, e));
    }
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
            return Ok(bytes);
        }
    }
    Err("Could not locate a system font".into())
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

    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let mut renderer = Renderer::with_graph(320, 240, &mut ctx, graph).expect("renderer");

    let font_bytes = load_system_font().unwrap_or_else(|e| {
        eprintln!("{}", e);
        eprintln!("Set KOJI_FONT_PATH to a valid .ttf font to run text tests.");
        panic!("font not found");
    });
    renderer.fonts_mut().register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(renderer.fonts(), "default");
    let info = StaticTextCreateInfo { text: "Hello", scale: 32.0, pos: [-0.5, 0.5], key: "glyph_tex", screen_size: [320.0, 240.0], color: [1.0; 4], bold: false, italic: false };
    let mesh = StaticText::new(&mut ctx, renderer.resources(), &mut text, info).unwrap();
    renderer.register_text_mesh(mesh, "canvas");
    text.register_textures(renderer.resources());

    let vert_spv = make_vert();
    let frag_spv = make_frag();
    let mut pso = PipelineBuilder::new(&mut ctx, "text_pso")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass(renderer.graph().output("color"))
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
