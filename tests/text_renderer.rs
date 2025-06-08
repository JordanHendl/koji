use koji::text::TextRenderer2D;
use koji::utils::{ResourceManager, ResourceBinding};
use dashi::gpu;
use rusttype::{Font, Scale, point};
use serial_test::serial;

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
    let font_bytes: &[u8] = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
    let text = TextRenderer2D::new(font_bytes);
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let dim = text.upload_text_texture(&mut ctx, &mut res, "hello", "Hi", 20.0);
    assert_eq!(dim, expected_dims("Hi", 20.0, font_bytes));
    destroy_combined(&mut ctx, &res, "hello");
    ctx.destroy();
}

#[test]
#[serial]
fn upload_registers_texture_with_expected_dims() {
    let font_bytes: &[u8] = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
    let text = TextRenderer2D::new(font_bytes);
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();

    let dim = text.upload_text_texture(&mut ctx, &mut res, "greeting", "Hello", 32.0);
    let expected = expected_dims("Hello", 32.0, font_bytes);
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
    let font_bytes: &[u8] = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
    let text = TextRenderer2D::new(font_bytes);
    let dim = [16, 8];
    let pos = [1.0, 2.0];
    let mesh = text.make_quad(dim, pos);
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
#[should_panic]
fn upload_empty_string_zero_texture() {
    let font_bytes: &[u8] = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
    let text = TextRenderer2D::new(font_bytes);
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let dim = text.upload_text_texture(&mut ctx, &mut res, "empty", "", 16.0);
    assert_eq!(dim[0], 0);
    match res.get("empty") {
        Some(ResourceBinding::CombinedImageSampler { texture, .. }) => {
            assert_eq!(texture.dim[0], 0);
        }
        _ => panic!("expected combined sampler"),
    }
    destroy_combined(&mut ctx, &res, "empty");
    ctx.destroy();
}
