use dashi::*;
use dashi::utils::Handle;
use inline_spirv::include_spirv;
use koji::canvas::CanvasBuilder;
use koji::material::pipeline_builder::PipelineBuilder;
use koji::renderer::*;
use koji::text::*;
use std::cell::RefCell;
use std::rc::Rc;
use winit::event::{
    ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent,
};

fn load_system_font() -> Result<Vec<u8>, String> {
    if let Ok(path) = std::env::var("KOJI_FONT_PATH") {
        return std::fs::read(&path)
            .map_err(|e| format!("Failed to read font at {}: {}", path, e));
    }
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

struct SharedDynamic(Rc<RefCell<DynamicText>>);

impl TextRenderable for SharedDynamic {
    fn vertex_buffer(&self) -> Handle<Buffer> {
        self.0.borrow().vertex_buffer()
    }

    fn index_buffer(&self) -> Option<Handle<Buffer>> {
        Some(self.0.borrow().index_buffer())
    }

    fn index_count(&self) -> usize {
        self.0.borrow().index_count
    }
}

#[cfg(feature = "gpu_tests")]
pub fn run(ctx: &mut Context) {
    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(ctx)
        .unwrap();

    let mut renderer = Renderer::with_canvas(320, 240, ctx, canvas).expect("renderer");

    let font_bytes = load_system_font().unwrap_or_else(|e| {
        eprintln!("{}", e);
        eprintln!("Set KOJI_FONT_PATH to a valid .ttf font to run this example.");
        std::process::exit(1);
    });
    renderer.fonts_mut().register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(renderer.fonts(), "default");
    // Static text shown at the top left
    let static_text = StaticText::new(
        ctx,
        renderer.resources(),
        &mut text,
        StaticTextCreateInfo {
            text: "Static text",
            scale: 32.0,
            pos: [-0.9, 0.9],
            key: "glyph_tex",
            screen_size: [320.0, 240.0],
            color: [1.0; 4],
            bold: false,
            italic: true,
        },
    ).unwrap();
    renderer.register_text_mesh(static_text, "canvas");

    // Dynamic text that updates with user input
    let dynamic = Rc::new(RefCell::new(
        DynamicText::new(
            ctx,
            &mut text,
            renderer.resources(),
            DynamicTextCreateInfo {
                max_chars: 64,
                text: "",
                scale: 32.0,
                pos: [-0.5, 0.5],
                key: "glyph_tex",
                screen_size: [320.0, 240.0],
                color: [1.0; 4],
                bold: true,
                italic: false,
            },
        )
        .expect("failed to create DynamicText"),
    ));
    renderer.register_text_mesh(SharedDynamic(dynamic.clone()), "canvas");
    text.register_textures(renderer.resources());
    let mut input = String::new();

    let vert_spv = make_vert();
    let frag_spv = make_frag();
    let mut pso = PipelineBuilder::new(ctx, "text_pso")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass(renderer.graph().output("color"))
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
            dynamic
                .borrow_mut()
                .update_text(ctx, r.resources(), &mut text, &input, 32.0, [-0.5, 0.5])
                .unwrap();
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
