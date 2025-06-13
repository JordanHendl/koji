// src/deferred_render.rs
use dashi::utils::*;
use dashi::*;
use inline_spirv::include_spirv;
use koji::render_pass::*;
use koji::*;
use koji::utils::ResourceManager;
use winit::event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
// Shader sources live in `assets/shaders/` and are included with `include_spirv!`.
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
            "assets/shaders/deferred.vert",
            vert
        ),
        specialization: &[],
    };

    let frag_gbuffer = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: include_spirv!(
            "assets/shaders/gbuffer.frag",
            frag
        ),
        specialization: &[],
    };

    let frag_lighting = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: include_spirv!(
            "assets/shaders/lighting.frag",
            frag
        ),
        specialization: &[],
    };

    let pso_gbuffer = PipelineBuilder::new(ctx, "gbuffer")
        .vertex_shader(vert.spirv)
        .fragment_shader(frag_gbuffer.spirv)
        .render_pass(render_pass, 0)
        .build();

    let mut pso_lighting = PipelineBuilder::new(ctx, "lighting")
        .vertex_shader(vert.spirv)
        .fragment_shader(frag_lighting.spirv)
        .render_pass(render_pass, 1)
        .build();

    let pipeline_gbuffer = pso_gbuffer.pipeline;
    let _pipeline_lighting = pso_lighting.pipeline;

    let sampler = ctx.make_sampler(&Default::default()).unwrap();
    let mut resources = ResourceManager::new(ctx, 4096).unwrap();
    resources.register_combined(
        "albedoTex",
        Handle::default(),
        gbuffer_target.colors[0].attachment.img,
        [WIDTH, HEIGHT],
        sampler,
    );
    resources.register_combined(
        "normalTex",
        Handle::default(),
        gbuffer_target.colors[1].attachment.img,
        [WIDTH, HEIGHT],
        sampler,
    );

    let mut lights = BindlessLights::new();
    lights.add_light(
        ctx,
        &mut resources,
        LightDesc { position: [0.0, 0.0, 1.0], intensity: 1.0, color: [1.0, 1.0, 1.0], _pad: 0, ..Default::default() },
    );
    lights.add_light(
        ctx,
        &mut resources,
        LightDesc { position: [1.0, 1.0, 1.0], intensity: 0.5, color: [1.0, 0.0, 0.0], _pad: 0, ..Default::default() },
    );
    let light_count = lights.lights.lock().unwrap().len();
    lights.register(&mut resources);
    resources.register_variable("", ctx, light_count);

    let lighting_bg = pso_lighting.create_bind_group(0, &resources).unwrap();

    let mut display = ctx.make_display(&Default::default()).unwrap();
    let semaphores = ctx.make_semaphores(2).unwrap();
    let mut framed = FramedCommandList::new(ctx, "deferred", 2);
    let _timer = Instant::now();

    'main: loop {
        let mut should_exit = false;
        {
            let event_loop = display.winit_event_loop();
            event_loop.run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Exit;
                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        WindowEvent::CloseRequested |
                        WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), state: ElementState::Pressed, .. }, .. } =>
                            should_exit = true,
                        _ => {}
                    }
                }
            });
        }
        if should_exit {
            break 'main;
        }

        let (img, acquire_sem, _, _) = ctx.acquire_new_image(&mut display).unwrap();

        framed.record(|cmd| {
            cmd.begin_drawing(&DrawBegin {
                pipeline: pipeline_gbuffer,
                viewport: Viewport {
                    area: FRect2D { w: 640.0, h: 480.0, ..Default::default() },
                    scissor: Rect2D { w: 640, h: 480, ..Default::default() },
                    ..Default::default()
                },
                attachments: &attachments
                    .attachments
                    .iter()
                    .map(|a| a.attachment.clone())
                    .collect::<Vec<_>>(),
            }).unwrap();

            cmd.draw(Draw {
                vertices: vertex_buffer,
                count: 3,
                instance_count: 1,
                ..Default::default()
            });

            cmd.next_subpass().unwrap();

            cmd.draw(Draw {
                vertices: vertex_buffer,
                count: 3,
                instance_count: 1,
                bind_groups: [Some(lighting_bg.bind_group), None, None, None],
                ..Default::default()
            });

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
    ctx.destroy();
}
