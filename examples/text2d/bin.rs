use dashi::*;
use inline_spirv::include_spirv;
use koji::material::pipeline_builder::PipelineBuilder;
use koji::renderer::*;
use koji::text::*;
use winit::event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};

fn load_system_font() -> Vec<u8> {
    #[cfg(target_os = "windows")]
    const CANDIDATES: &[&str] = &["C:/Windows/Fonts/arial.ttf", "C:/Windows/Fonts/segoeui.ttf"];
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
pub fn run(ctx: &mut Context) {
    let mut renderer = Renderer::new(320, 240, "text", ctx).expect("renderer");

    let font_bytes = load_system_font();
    renderer.fonts_mut().register_font("default", &font_bytes);
    let text = TextRenderer2D::new(renderer.fonts(), "default");
    let mut input = String::new();
    let dim = text.upload_text_texture(ctx, renderer.resources(), "glyph_tex", &input, 32.0);
    let mesh = text.make_quad(dim, [-0.5, 0.5]);
    renderer.register_text_mesh(mesh);
    let mesh_idx = 0usize;

    let vert_spv = make_vert();
    let frag_spv = make_frag();
    let mut pso = PipelineBuilder::new(ctx, "text_pso")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass(renderer.render_pass(), 0)
        .build();
    let bgr = pso.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_pso(RenderStage::Text, pso, bgr);

    renderer.render_loop(|r, event| {
        let mut changed = false;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::ReceivedCharacter(c) => {
                    if !c.is_control() {
                        input.push(c);
                        changed = true;
                    }
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Back),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    input.pop();
                    changed = true;
                }
                _ => {}
            },
            Event::MainEventsCleared => {}
            _ => {}
        }

        if changed {
            let dim = text.upload_text_texture(ctx, r.resources(), "glyph_tex", &input, 32.0);
            let mesh = text.make_quad(dim, [-0.5, 0.5]);
            r.update_text_mesh(mesh_idx, mesh);
        }
    });
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    {
        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = Context::new(&ContextInfo { device }).unwrap();
        run(&mut ctx);
        ctx.destroy();
    }
}
