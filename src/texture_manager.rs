use crate::utils::{ResourceBinding, ResourceManager, Texture};
use dashi::utils::Handle;
use dashi::{Context, Format, ImageInfo, ImageViewInfo};
use image::GenericImageView;

/// Load a PNG texture from memory and register it with the [`ResourceManager`].
/// Returns a handle into `ResourceManager::textures`.
pub fn load_from_bytes(
    ctx: &mut Context,
    res: &mut ResourceManager,
    key: &str,
    fmt: dashi::Format,
    bytes: &[u8],
) -> Handle<Texture> {
    let img = image::load_from_memory(bytes).expect("Failed to decode image");
    let rgba = img.to_rgba8();
    let (w, h) = img.dimensions();

    let image = ctx
        .make_image(&ImageInfo {
            debug_name: key,
            dim: [w, h, 1],
            layers: 1,
            format: fmt,
            mip_levels: 12,
            initial_data: Some(&rgba),
        })
        .unwrap();

    let view = ctx
        .make_image_view(&ImageViewInfo {
            img: image,
            ..Default::default()
        })
        .unwrap();

    let tex = Texture {
        handle: image,
        view,
        dim: [w, h],
    };
    let handle = res.textures.push(tex.clone());
    res.bindings
        .insert(key.into(), ResourceBinding::Texture(tex));
    handle
}

/// Load a PNG texture from a file path and register it with the [`ResourceManager`].
pub fn load_from_file(
    ctx: &mut Context,
    res: &mut ResourceManager,
    key: &str,
    fmt: dashi::Format,
    path: &std::path::Path,
) -> Handle<Texture> {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|_| panic!("Failed to read texture file {}", path.display()));
    load_from_bytes(ctx, res, key, fmt, &bytes)
}

/// Create a single 1x1 texture with a solid RGBA color and register it with the [`ResourceManager`].
pub fn create_solid_color(
    ctx: &mut Context,
    res: &mut ResourceManager,
    key: &str,
    color: [u8; 4],
) -> Handle<Texture> {
    let image = ctx
        .make_image(&ImageInfo {
            debug_name: key,
            dim: [1, 1, 1],
            layers: 1,
            format: Format::RGBA8,
            mip_levels: 12,
            initial_data: Some(&color),
        })
        .unwrap();

    let view = ctx
        .make_image_view(&ImageViewInfo {
            img: image,
            ..Default::default()
        })
        .unwrap();

    let tex = Texture {
        handle: image,
        view,
        dim: [1, 1],
    };

    let handle = res.textures.push(tex.clone());
    res.bindings
        .insert(key.into(), ResourceBinding::Texture(tex));
    handle
}

/// Free a texture previously loaded via this module.
pub fn free_texture(ctx: &mut Context, res: &mut ResourceManager, handle: Handle<Texture>) {
    if !handle.valid() {
        return;
    }

    if let Some(&tex) = res.textures.pool.get_ref(handle) {
        ctx.destroy_image_view(tex.view);
        ctx.destroy_image(tex.handle);
        res.textures.release(handle);

        if let Some(key) = res
            .bindings
            .iter()
            .find(|(_, b)| match b {
                ResourceBinding::Texture(t) => t.handle == tex.handle,
                ResourceBinding::CombinedImageSampler { texture, .. } => {
                    texture.handle == tex.handle
                }
                _ => false,
            })
            .map(|(k, _)| k.clone())
        {
            res.bindings.remove(&key);
        }
    }
}
