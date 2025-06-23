use koji::texture_manager::{load_from_bytes, free_texture};
use koji::utils::{ResourceManager, ResourceBinding, Texture};
use dashi::gpu;
use serial_test::serial;
use image::{RgbaImage, Rgba, ImageOutputFormat};
use std::io::Cursor;

fn setup_ctx() -> gpu::Context {
    gpu::Context::headless(&Default::default()).unwrap()
}

fn in_memory_png() -> Vec<u8> {
    let img = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut cursor, ImageOutputFormat::Png)
        .unwrap();
    cursor.into_inner()
}

#[test]
#[serial]
fn load_adds_binding_and_texture_entry() {
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();
    let bytes = in_memory_png();
    let handle = load_from_bytes(&mut ctx, &mut res, "mem_tex", Default::default(), &bytes);

    assert_eq!(res.textures.entries.len(), 1);
    assert!(res.bindings.contains_key("mem_tex"));
    match res.get("mem_tex") {
        Some(ResourceBinding::Texture(tex)) => assert_eq!(tex.dim, [1, 1]),
        _ => panic!("expected texture binding"),
    }

    free_texture(&mut ctx, &mut res, handle);
    assert!(res.textures.entries.is_empty());
    assert!(res.get("mem_tex").is_none());
    ctx.destroy();
}

#[test]
#[serial]
fn free_invalid_handle_does_not_panic() {
    let mut ctx = setup_ctx();
    let mut res = ResourceManager::default();

    let invalid = dashi::utils::Handle::<Texture>::default();
    free_texture(&mut ctx, &mut res, invalid);
    ctx.destroy();
}
