#![cfg(feature = "gpu_tests")]

use koji::text::{
    TextRenderer2D, FontRegistry, StaticText, StaticTextCreateInfo, DynamicText,
    DynamicTextCreateInfo,
};
use koji::utils::{ResourceManager, ResourceBinding};
use dashi::gpu;
use rusttype::{Font, Scale, point};
use serial_test::serial;

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

fn setup_ctx() -> gpu::Context {
    gpu::Context::headless(&Default::default()).unwrap()
}

fn destroy_combined(ctx: &mut gpu::Context, res: &ResourceManager, key: &str) {
    if let Some(ResourceBinding::CombinedImageSampler { texture, .. }) = res.get(key) {
        ctx.destroy_image_view(texture.view);
        ctx.destroy_image(texture.handle);
    }
}

fn expected_dims(text: &str, scale: f32, font_bytes: &[u8]) -> [u32; 2] {
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

#[test]
#[serial]
fn new_loads_font_bytes() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let (_idx, dim) = text
        .upload_text_texture(&mut ctx, &mut res, "hello", "Hi", 20.0)
        .unwrap();
    assert_eq!(dim, expected_dims("Hi", 20.0, &font_bytes));
    destroy_combined(&mut ctx, &res, "hello");
    ctx.destroy();
}

#[test]
#[serial]
fn upload_registers_texture_with_expected_dims() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();

    let (_idx, dim) = text
        .upload_text_texture(&mut ctx, &mut res, "greeting", "Hello", 32.0)
        .unwrap();
    let expected = expected_dims("Hello", 32.0, &font_bytes);
    assert_eq!(dim, expected);
    match res.get("greeting") {
        Some(ResourceBinding::CombinedImageSampler { texture, .. }) => {
            assert_eq!(texture.dim, expected);
        }
        _ => panic!("expected combined sampler"),
    }
    destroy_combined(&mut ctx, &res, "greeting");
    ctx.destroy();
}

#[test]
fn make_quad_generates_correct_vertices() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let dim = [16, 8];
    let pos = [1.0, 2.0];
    let mesh = text.make_quad(dim, pos, 0);
    let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();
    assert_eq!(positions, vec![
        [1.0, 2.0 - 8.0, 0.0],
        [1.0 + 16.0, 2.0 - 8.0, 0.0],
        [1.0 + 16.0, 2.0, 0.0],
        [1.0, 2.0, 0.0],
    ]);
    let uvs: Vec<[f32; 2]> = mesh.vertices.iter().map(|v| v.uv).collect();
    assert_eq!(uvs, vec![
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ]);
    assert_eq!(mesh.indices, Some(vec![0, 1, 2, 2, 3, 0]));
}

#[test]
#[serial]
fn upload_empty_string_zero_texture() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let (_idx, dim) = text
        .upload_text_texture(&mut ctx, &mut res, "empty", "", 16.0)
        .unwrap();
    assert_eq!(dim, [1, 1]);
    match res.get("empty") {
        Some(ResourceBinding::CombinedImageSampler { texture, .. }) => {
            assert_eq!(texture.dim, [1, 1]);
        }
        _ => panic!("expected combined sampler"),
    }
    destroy_combined(&mut ctx, &res, "empty");
    ctx.destroy();
}

#[test]
#[serial]
fn static_text_preserves_gpu_buffers() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let info = StaticTextCreateInfo { text: "Hi", scale: 16.0, pos: [0.0, 0.0], key: "stex" };
    let mut st = StaticText::new(&mut ctx, &mut res, &mut text, info).unwrap();
    let vb = st.vertex_buffer();
    let ib = st.index_buffer().expect("ib");

    // modify a non-buffer field
    st.texture_key = "stex".into();

    assert_eq!(st.vertex_buffer(), vb);
    assert_eq!(st.index_buffer().unwrap(), ib);

    destroy_combined(&mut ctx, &res, "stex");
    ctx.destroy_buffer(vb);
    ctx.destroy_buffer(ib);
    ctx.destroy();
}

#[test]
#[serial]
#[ignore]
fn dynamic_text_updates_vertices_and_respects_capacity() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let info = DynamicTextCreateInfo { max_chars: 8, text: "hi", scale: 16.0, pos: [0.0, 0.0], key: "dtex", screen_size: [320.0, 240.0] };
    let mut dt = DynamicText::new(&mut ctx, &mut text, &mut res, info).unwrap();
    let vb = dt.vertex_buffer();
    assert_eq!(dt.vertex_count, 4);

    // update string within capacity
    dt.update_text(&mut ctx, &mut res, &mut text, "bye", 16.0, [0.0, 0.0]).unwrap();
    assert_eq!(dt.vertex_buffer(), vb);
    assert_eq!(dt.vertex_count, 4);
    let expected_dim = expected_dims("bye", 16.0, &font_bytes);
    match res.get("dtex") {
        Some(ResourceBinding::CombinedImageSampler { texture, .. }) => {
            assert_eq!(texture.dim, expected_dim);
        }
        _ => panic!("expected combined sampler"),
    }

    destroy_combined(&mut ctx, &res, "dtex");
    dt.destroy(&mut ctx);
    ctx.destroy();
}

#[test]
#[serial]
fn dynamic_text_update_empty_string_resets_counts() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let info = DynamicTextCreateInfo { max_chars: 8, text: "hello", scale: 16.0, pos: [0.0, 0.0], key: "emt", screen_size: [320.0, 240.0] };
    let mut dt = DynamicText::new(&mut ctx, &mut text, &mut res, info).unwrap();
    dt.update_text(&mut ctx, &mut res, &mut text, "", 16.0, [0.0, 0.0]).unwrap();
    assert_eq!(dt.vertex_count, 0);
    assert_eq!(dt.index_count, 0);
    destroy_combined(&mut ctx, &res, "emt");
    dt.destroy(&mut ctx);
    ctx.destroy();
}

#[test]
#[serial]
#[ignore]
fn dynamic_text_update_over_capacity_panics() {
    let font_bytes = load_system_font();
    let mut registry = FontRegistry::new();
    registry.register_font("default", &font_bytes);
    let mut text = TextRenderer2D::new(&registry, "default");
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let info = DynamicTextCreateInfo { max_chars: 2, text: "hi", scale: 16.0, pos: [0.0, 0.0], key: "ovr", screen_size: [320.0, 240.0] };
    let mut dt = DynamicText::new(&mut ctx, &mut text, &mut res, info).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dt.update_text(&mut ctx, &mut res, &mut text, "toolong", 16.0, [0.0, 0.0])
    }));
    match result {
        Ok(Err(_)) => {},
        Err(_) => {},
        _ => panic!("update should fail"),
    }
    destroy_combined(&mut ctx, &res, "ovr");
    dt.destroy(&mut ctx);
    ctx.destroy();
}
