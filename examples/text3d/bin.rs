use dashi::*;
use inline_spirv::include_spirv;
use koji::canvas::CanvasBuilder;
use koji::material::pipeline_builder::PipelineBuilder;
use koji::renderer::*;
use koji::text::*;
use glam::*;
use winit::event::Event;

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

#[cfg(feature = "gpu_tests")]
pub fn run(ctx: &mut Context) {
    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(ctx)
        .unwrap();

    let mut renderer = Renderer::with_canvas(320, 240, ctx, canvas).expect("renderer");
    renderer.set_clear_depth(1.0);

    let font_bytes = load_system_font().unwrap_or_else(|e| {
        eprintln!("{}", e);
        eprintln!("Set KOJI_FONT_PATH to a valid .ttf font to run this example.");
        std::process::exit(1);
    });
    renderer.fonts_mut().register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(renderer.fonts(), "default");

    let (_idx, dim) = text
        .upload_text_texture(ctx, renderer.resources(), "glyph_tex", "3D Text", 32.0)
        .unwrap();
    let proj = Mat4::perspective_rh_gl(45_f32.to_radians(), 320.0 / 240.0, 0.1, 10.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 2.0), Vec3::ZERO, Vec3::Y);
    let mat = proj * view * Mat4::IDENTITY;
    let mesh = text.make_quad_3d(dim, mat, _idx, [1.0; 4], true);
    renderer.register_text_mesh(mesh);
    text.register_textures(renderer.resources());
    let mesh_idx = 0usize;

    let vert_spv = make_vert();
    let frag_spv = make_frag();
    let mut pso = PipelineBuilder::new(ctx, "text3d_pso")
        .vertex_shader(&vert_spv)
        .fragment_shader(&frag_spv)
        .render_pass(renderer.graph().output("color"))
        .build_with_resources(renderer.resources())
        .unwrap();
    let bgr = pso.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_pso(RenderStage::Text, pso, bgr);

    let mut angle: f32 = 0.0;
    renderer.render_loop(|r, event| {
        if let Event::MainEventsCleared = event {
            angle += 0.01;
            let mat = proj
                * view
                * Mat4::from_rotation_y(angle);
            let mesh2 = text.make_quad_3d(dim, mat, _idx, [1.0; 4], true);
            r.update_text_mesh(mesh_idx, mesh2);
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
