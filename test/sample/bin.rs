use dashi::utils::*;
use dashi::*;
use koji::*;
use sdl2::{event::Event, keyboard::Keycode};

pub fn main() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();

    let mut ctx = Context::new(&ContextInfo { device }).unwrap();

    // Generate a basic render pass with 1 color attachment
    let (rp, targets, attachments) = RenderPassBuilder::new()
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
    // Create vertex buffer for a triangle
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

    // Create graphics pipeline layout
    let pipeline_layout = ctx
        .make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
            debug_name: "SamplePipelineLayout",
            vertex_info: VertexDescriptionInfo {
                entries: &[VertexEntryInfo {
                    format: ShaderPrimitiveType::Vec2,
                    location: 0,
                    offset: 0,
                }],
                stride: 8,
                rate: VertexRate::Vertex,
            },
            bg_layouts: [None, None, None, None],
            shaders: &[
                PipelineShaderInfo {
                    stage: ShaderType::Vertex,
                    spirv: inline_spirv::inline_spirv!(
                        r#"
                    #version 450
                    layout(location = 0) in vec2 inPosition;
                    layout(location = 0) out vec3 inColor;
                    void main() {
                        inColor = vec3(inPosition.x, inPosition.y, 0.0);
                        gl_Position = vec4(inPosition, 0.0, 1.0);
                    }
                "#,
                        vert
                    ),
                    specialization: &[],
                },
                PipelineShaderInfo {
                    stage: ShaderType::Fragment,
                    spirv: inline_spirv::inline_spirv!(
                        r#"
                    #version 450
                    layout(location = 0) in vec3 inColor;
                    layout(location = 0) out vec4 outColor;
                    void main() {
                        outColor = vec4(inColor.x, inColor.y, inColor.z, 1.0);
                    }
                "#,
                        frag
                    ),
                    specialization: &[],
                },
            ],
            details: Default::default(),
        })
        .unwrap();

    // Create graphics pipeline
    let pipeline = ctx
        .make_graphics_pipeline(&GraphicsPipelineInfo {
            debug_name: "SamplePipeline",
            layout: pipeline_layout,
            render_pass: rp,
            subpass_id: 0,
        })
        .unwrap();

    // Setup SDL2 display
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

        let (img, acquire_sem, _img_idx, _ok) = ctx.acquire_new_image(&mut display).unwrap();

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
                    pipeline,
                    attachments: &target
                        .colors
                        .iter()
                        .map(|a| a.attachment.clone())
                        .collect::<Vec<_>>(),
                })
                .unwrap();

                list.append(Command::Draw(Draw {
                    vertices: vertex_buffer,
                    count: 3,
                    instance_count: 1,
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
