#![cfg(feature = "gpu_tests")]

use koji::text::{TextRenderer2D, StaticText, StaticTextCreateInfo, DynamicText, DynamicTextCreateInfo, FontRegistry};
use koji::utils::{ResourceManager, ResourceBinding};
use dashi::gpu;
use serial_test::serial;

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

fn setup_ctx() -> gpu::Context {
    gpu::Context::headless(&Default::default()).unwrap()
}

fn destroy_combined(ctx: &mut gpu::Context, res: &ResourceManager, key: &str) {
    if let Some(ResourceBinding::CombinedImageSampler { texture, .. }) = res.get(key) {
        ctx.destroy_image_view(texture.view);
        ctx.destroy_image(texture.handle);
    }
}


#[test]
#[serial]
fn static_text_new_uploads_texture() {
    let font_bytes = load_system_font().unwrap_or_else(|e| {
        eprintln!("{}", e);
        eprintln!("Set KOJI_FONT_PATH to a valid .ttf font to run text tests.");
        panic!("font not found");
    });
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let info = StaticTextCreateInfo {
        text: "Hi",
        scale: 16.0,
        pos: [0.0, 0.0],
        key: "stex",
        screen_size: [320.0, 240.0],
        color: [1.0; 4],
        bold: false,
        italic: false,
    };
    let s = StaticText::new(&mut ctx, &mut res, &mut text, info).unwrap();
    assert_eq!(s.dim()[0] > 0, true);
    assert!(res.get("stex").is_some());
    destroy_combined(&mut ctx, &res, "stex");
    if let Some(ib) = s.index_buffer() { ctx.destroy_buffer(ib); }
    ctx.destroy_buffer(s.vertex_buffer());
    ctx.destroy();
}

#[test]
#[serial]
#[ignore]
fn dynamic_text_update_respects_max_chars() {
    let font_bytes = load_system_font().unwrap_or_else(|e| {
        eprintln!("{}", e);
        eprintln!("Set KOJI_FONT_PATH to a valid .ttf font to run text tests.");
        panic!("font not found");
    });
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let info = DynamicTextCreateInfo { max_chars: 4, text: "hey", scale: 16.0, pos: [0.0, 0.0], key: "dtex", screen_size: [320.0, 240.0], color: [1.0; 4], bold: false, italic: false };
    let mut d = DynamicText::new(&mut ctx, &mut text, &mut res, info).unwrap();
    assert_eq!(d.vertex_count, 4);
    assert!(res.get("dtex").is_some());
    d.update_text(&mut ctx, &mut res, &mut text, "hi", 16.0, [0.0, 0.0]).unwrap();
    destroy_combined(&mut ctx, &res, "dtex");
    d.destroy(&mut ctx);
    ctx.destroy();
}
