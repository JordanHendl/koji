use dashi::utils::*;
use dashi::*;
use koji::*;
use sdl2::{event::Event, keyboard::Keycode};
use std::sync::Arc;

pub fn main() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();

    let mut ctx = Context::new(&ContextInfo { device }).unwrap();

    // Generate a basic render pass with 1 color attachment
    let (rp, targets, _attachments) = RenderPassBuilder::new()
        .debug_name("Sample Render Pass")
        .extent([640, 480])
        .viewport(Viewport {
            area: FRect2D {
                w: 640.0,
                h: 480.0,
                ..Default::default()
            },
            scissor: Rect2D {
                w: 640,
                h: 480,
                ..Default::default()
            },
            ..Default::default()
        })
        .color_attachment("color", Format::RGBA8)
        .subpass("subpass1", &["color"], &[] as &[&str])
        .build_with_images(&mut ctx)
        .unwrap();

    render_sample_model(&mut ctx, rp, &targets);
    ctx.destroy();
}

pub fn render_sample_model(ctx: &mut Context, rp: Handle<RenderPass>, targets: &[RenderTarget]) {
    // Vertex buffer for a triangle
    const VERTICES: [[f32; 2]; 3] = [[0.0, -0.5], [0.5, 0.5], [-0.5, 0.5]];
    let vertex_buffer = ctx
        .make_buffer(&BufferInfo {
            debug_name: "triangle_vertices",
            byte_size: (VERTICES.len() * std::mem::size_of::<f32>() * 2) as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::VERTEX,
            initial_data: unsafe { Some(VERTICES.align_to::<u8>().1) },
        })
        .unwrap();

    // ==== NEW: Create texture and upload a single-pixel image ====
    let tex_data: [u8; 4] = [255, 0, 0, 255];
    let img = ctx.make_image(&ImageInfo {
        debug_name: "sample_tex",
        dim: [1, 1, 1],
        format: Format::RGBA8,
        mip_levels: 1,
        layers: 1,
        initial_data: Some(&tex_data),
    }).unwrap();
    let view = ctx.make_image_view(&ImageViewInfo {
        img,
        debug_name: "sample_tex_view",
        ..Default::default()
    }).unwrap();
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();

    // ==== NEW: Create uniform buffer ====
    let uniform_value: f32 = 0.7;

    // ==== NEW: Set up PipelineBuilder shaders ====
    let vert_spirv = inline_spirv::inline_spirv!(
        r#"
        #version 450
        layout(location = 0) in vec2 inPosition;
        layout(location = 0) out vec2 uv;
        void main() {
            uv = inPosition * 0.5 + vec2(0.5, 0.5);
            gl_Position = vec4(inPosition, 0.0, 1.0);
        }
        "#,
        vert
    ).to_vec();

    let frag_spirv = inline_spirv::inline_spirv!(
        r#"
        #version 450
        layout(set = 0, binding = 0) uniform sampler2D tex;
        layout(set = 0, binding = 1) uniform UBO { float v; } ubo;
        layout(location = 0) in vec2 uv;
        layout(location = 0) out vec4 outColor;
        void main() {
            vec4 color = texture(tex, uv);
            outColor = mix(color, vec4(0.0, 1.0, 0.0, 1.0), ubo.v);
        }
        "#,
        frag
    ).to_vec();

    let mut pso = PipelineBuilder::new(ctx, "sample_pso")
        .vertex_shader(&vert_spirv)
        .fragment_shader(&frag_spirv)
        .render_pass(rp, 0)
        .build();

    // ==== NEW: Use ResourceManager to bind resources by shader name ====
    let mut resources = ResourceManager::new(ctx, 4096).unwrap();
    resources.register_combined("tex", img, view, [1, 1], sampler);
    resources.register_variable("ubo", ctx, uniform_value);

    let bind_group = pso.create_bind_group(0, &resources);

    // ==== The rest: draw with pipeline ====
    let mut display = ctx.make_display(&Default::default()).unwrap();
    let mut event_pump = ctx.get_sdl_ctx().event_pump().unwrap();
    let mut framed_list = FramedCommandList::new(ctx, "SampleRenderList", 2);
    let semaphores = ctx.make_semaphores(2).unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let (img, acquire_sem, img_idx, _ok) = ctx.acquire_new_image(&mut display).unwrap();

        framed_list.record(|list| {
            for target in targets {
                list.begin_drawing(&DrawBegin {
                    viewport: Viewport {
                        area: FRect2D {
                            w: 640.0,
                            h: 480.0,
                            ..Default::default()
                        },
                        scissor: Rect2D {
                            w: 640,
                            h: 480,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    pipeline: pso.pipeline,
                    attachments: &target
                        .colors
                        .iter()
                        .map(|a| a.attachment.clone())
                        .collect::<Vec<_>>(),
                })
                .unwrap();

                list.append(Command::Draw(Draw {
                    count: 3,
                    instance_count: 1,
                    vertices: vertex_buffer,
                    bind_groups: [Some(bind_group.bind_group), None, None, None],
                    ..Default::default()
                }));

                list.end_drawing().unwrap();

                list.blit_image(ImageBlit {
                    src: target.colors[0].attachment.img,
                    dst: img,
                    filter: Filter::Nearest,
                    ..Default::default()
                });
            }
        });

        framed_list.submit(&SubmitInfo {
            wait_sems: &[acquire_sem],
            signal_sems: &[semaphores[0], semaphores[1]],
        });

        ctx.present_display(&display, &[semaphores[0], semaphores[1]])
            .unwrap();
    }
}

