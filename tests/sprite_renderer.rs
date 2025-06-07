use koji::sprite::*;
use koji::sprite::renderer::SpriteRenderer;
use koji::utils::*;
use dashi::*;
use serial_test::serial;

#[test]
#[serial]
#[ignore]
fn render_textured_quad() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();

    let mut renderer = SpriteRenderer::new(320, 240, &mut ctx).expect("sprite renderer");

    let white: [u8; 4] = [255, 255, 255, 255];
    let img = ctx
        .make_image(&ImageInfo {
            debug_name: "white",
            dim: [1, 1, 1],
            format: Format::RGBA8,
            mip_levels: 1,
            layers: 1,
            initial_data: Some(&white),
        })
        .unwrap();
    let view = ctx
        .make_image_view(&ImageViewInfo { img, ..Default::default() })
        .unwrap();
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();

    renderer
        .resources()
        .register_combined("tex", img, view, [1, 1], sampler);
    renderer.update_bind_groups();

    let sprite = Sprite {
        vertices: vec![
            SpriteVertex { position: [-0.5, -0.5], uv: [0.0, 0.0] },
            SpriteVertex { position: [0.5, -0.5], uv: [1.0, 0.0] },
            SpriteVertex { position: [0.5, 0.5], uv: [1.0, 1.0] },
            SpriteVertex { position: [-0.5, 0.5], uv: [0.0, 1.0] },
        ],
        indices: Some(vec![0, 1, 2, 2, 3, 0]),
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    renderer.register_sprite(sprite);

    renderer.draw_sprites().unwrap();

    ctx.destroy();
}
