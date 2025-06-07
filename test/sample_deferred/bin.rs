// src/deferred_render.rs
use dashi::*;
use inline_spirv::include_spirv;
use koji::render_pass::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
// Shader sources live in `shaders/` and are included with `include_spirv!`.
use std::time::Instant;

pub fn run(ctx: &mut Context) {
    const WIDTH: u32 = 640;
    const HEIGHT: u32 = 480;
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

    let builder = RenderPassBuilder::new()
        .debug_name("deferred_pass")
        .extent([WIDTH, HEIGHT])
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
        .color_attachment("albedo", Format::RGBA8)
        .color_attachment("normal", Format::RGBA8)
        .color_attachment("lighting", Format::RGBA8)
        .depth_attachment("depth", Format::D24S8)
        .subpass(
            "gbuffer",
            ["albedo", "normal"],
            &[] as &[&str],
        )
        .subpass("lighting", ["lighting"], ["gbuffer"]);

    let (render_pass, targets, attachments) = builder.build_with_images(ctx).unwrap();

    let gbuffer_target = targets.iter().find(|t| t.name == "gbuffer").unwrap();
    let lighting_target = targets.iter().find(|t| t.name == "lighting").unwrap();

    let vert = PipelineShaderInfo {
        stage: ShaderType::Vertex,
        spirv: include_spirv!(
            "shaders/deferred.vert",
            vert
        ),
        specialization: &[],
    };

    let frag_gbuffer = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: include_spirv!(
            "shaders/gbuffer.frag",
            frag
        ),
        specialization: &[],
    };

    let frag_lighting = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: include_spirv!(
            "shaders/lighting.frag",
            frag
        ),
        specialization: &[],
    };

    let vertex_info = VertexDescriptionInfo {
        entries: &[VertexEntryInfo {
            format: ShaderPrimitiveType::Vec2,
            location: 0,
            offset: 0,
        }],
        stride: 8,
        rate: VertexRate::Vertex,
    };

    let pipeline_glayout = ctx
        .make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
            debug_name: "gbuffer_layout",
            vertex_info: vertex_info.clone(),
            bg_layouts: [None, None, None, None],
            shaders: &[vert.clone(), frag_gbuffer],
            details: GraphicsPipelineDetails {
                color_blend_states: vec![Default::default(), Default::default()],
                depth_test: Some(DepthInfo {
                    should_test: true,
                    should_write: true,
                }),
                ..Default::default()
            },
        })
        .unwrap();

    let pipeline_gbuffer = ctx
        .make_graphics_pipeline(&GraphicsPipelineInfo {
            layout: pipeline_glayout,
            render_pass,
            subpass_id: 0,
            debug_name: "gbuffer_pipeline",
        })
        .unwrap();

    let pipeline_llayout = ctx
        .make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
            debug_name: "lighting_layout",
            vertex_info,
            bg_layouts: [None, None, None, None],
            shaders: &[vert.clone(), frag_lighting],
            details: GraphicsPipelineDetails {
                depth_test: Some(DepthInfo {
                    should_test: true,
                    should_write: true,
                }),
                ..Default::default()
            },
        })
        .unwrap();

    let pipeline_lighting = ctx
        .make_graphics_pipeline(&GraphicsPipelineInfo {
            layout: pipeline_llayout,
            render_pass,
            subpass_id: 1,
            debug_name: "lighting_pipeline",
        })
        .unwrap();

    let mut display = ctx.make_display(&Default::default()).unwrap();
    let semaphores = ctx.make_semaphores(2).unwrap();
    let mut framed = FramedCommandList::new(ctx, "deferred", 2);
    let mut event_pump = ctx.get_sdl_ctx().event_pump().unwrap();
    let _timer = Instant::now();

    'main: loop {
        for e in event_pump.poll_iter() {
            if matches!(
                e,
                Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    }
            ) {
                break 'main;
            }
        }

        let (img, acquire_sem, _, _) = ctx.acquire_new_image(&mut display).unwrap();

        framed.record(|cmd| {
            for (pipeline, target) in [
                (pipeline_gbuffer, gbuffer_target),
                (pipeline_lighting, lighting_target),
            ] {
                cmd.begin_drawing(&DrawBegin {
                    pipeline,
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
                    attachments: &attachments
                        .attachments
                        .iter()
                        .map(|a| a.attachment.clone())
                        .collect::<Vec<_>>(),
                })
                .unwrap();

                cmd.draw(Draw {
                    vertices: vertex_buffer,
                    count: 3,
                    instance_count: 1,
                    ..Default::default()
                });

                if target.name != "lighting" {
                    cmd.next_subpass().unwrap();
                }
            }

            cmd.end_drawing().unwrap();
            cmd.blit_image(ImageBlit {
                src: lighting_target.colors[0].attachment.img,
                dst: img,
                filter: Filter::Nearest,
                ..Default::default()
            });
        });

        framed.submit(&SubmitInfo {
            wait_sems: &[acquire_sem],
            signal_sems: &semaphores,
        });

        ctx.present_display(&display, &semaphores).unwrap();
    }
}

pub fn main() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();

    let mut ctx = Context::new(&ContextInfo { device }).unwrap();

    run(&mut ctx);
}
