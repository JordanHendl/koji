use dashi::*;
use glam::*;
use inline_spirv::include_spirv;
use koji::render_pass::*;
use winit::event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
// Shader code is located in `assets/shaders/` and included via `include_spirv!`.

pub fn run(ctx: &mut Context) {
    const WIDTH: u32 = 640;
    const HEIGHT: u32 = 480;

    let light_direction = glam::Vec3::new(-1.0, -2.0, -1.0).normalize();
    let cascade_splits = [0.05, 0.15, 0.3, 1.0];

    println!("Running Cascaded Shadow Maps Example");
    println!("Light direction: {:?}", light_direction);
    println!("Cascade splits: {:?}", cascade_splits);

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct CameraUniform {
        pub view_proj: glam::Mat4,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Vertex {
        pub position: [f32; 3],
        pub normal: [f32; 3],
    }

    pub const CUBE_VERTICES: &[Vertex] = &[
        // Front
        Vertex {
            position: [-1.0, -1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [1.0, -1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [-1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
        },
        // Back
        Vertex {
            position: [-1.0, -1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [1.0, -1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [1.0, 1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [-1.0, 1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
        },
        // Left
        Vertex {
            position: [-1.0, -1.0, -1.0],
            normal: [-1.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, -1.0, 1.0],
            normal: [-1.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, 1.0, 1.0],
            normal: [-1.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, 1.0, -1.0],
            normal: [-1.0, 0.0, 0.0],
        },
        // Right
        Vertex {
            position: [1.0, -1.0, -1.0],
            normal: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, -1.0, 1.0],
            normal: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0, 1.0],
            normal: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0, -1.0],
            normal: [1.0, 0.0, 0.0],
        },
        // Top
        Vertex {
            position: [-1.0, 1.0, -1.0],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [-1.0, 1.0, 1.0],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0, 1.0],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0, -1.0],
            normal: [0.0, 1.0, 0.0],
        },
        // Bottom
        Vertex {
            position: [-1.0, -1.0, -1.0],
            normal: [0.0, -1.0, 0.0],
        },
        Vertex {
            position: [-1.0, -1.0, 1.0],
            normal: [0.0, -1.0, 0.0],
        },
        Vertex {
            position: [1.0, -1.0, 1.0],
            normal: [0.0, -1.0, 0.0],
        },
        Vertex {
            position: [1.0, -1.0, -1.0],
            normal: [0.0, -1.0, 0.0],
        },
    ];
    pub const CUBE_INDICES: &[u32] = &[
        // Front
        0, 1, 2, 0, 2, 3, // Back
        4, 6, 5, 4, 7, 6, // Left
        8, 9, 10, 8, 10, 11, // Right
        12, 14, 13, 12, 15, 14, // Top
        16, 17, 18, 16, 18, 19, // Bottom
        20, 22, 21, 20, 23, 22,
    ];

    let vertex_buffer = ctx
        .make_buffer(&BufferInfo {
            debug_name: "cube_vertices",
            byte_size: (CUBE_VERTICES.len() * std::mem::size_of::<Vertex>()) as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::VERTEX,
            initial_data: unsafe { Some(CUBE_VERTICES.align_to::<u8>().1) },
        })
        .unwrap();

    let index_buffer = ctx
        .make_buffer(&BufferInfo {
            debug_name: "cube_indices",
            byte_size: (CUBE_INDICES.len() * std::mem::size_of::<u32>()) as u32,
            visibility: MemoryVisibility::Gpu,
            usage: BufferUsage::INDEX,
            initial_data: unsafe { Some(CUBE_INDICES.align_to::<u8>().1) },
        })
        .unwrap();
    let viewport = Viewport {
        area: FRect2D {
            w: WIDTH as f32,
            h: HEIGHT as f32,
            ..Default::default()
        },
        scissor: Rect2D {
            w: WIDTH,
            h: HEIGHT,
            ..Default::default()
        },
        ..Default::default()
    };
    // -- Render Pass Setup --
    let builder = RenderPassBuilder::new()
        .debug_name("csm_pass")
        .extent([WIDTH, HEIGHT])
        .viewport(Viewport {
            area: FRect2D {
                w: WIDTH as f32,
                h: HEIGHT as f32,
                ..Default::default()
            },
            scissor: Rect2D {
                w: WIDTH,
                h: HEIGHT,
                ..Default::default()
            },
            ..Default::default()
        })
        .color_attachment("final", Format::RGBA8)
        .depth_attachment("depth", Format::D24S8)
        .subpass("shadow", &[] as &[&str], &[] as &[&str])
        .subpass("main", ["final"], ["shadow"]);

    let (render_pass, targets, attachments) = builder.build_with_images(ctx).unwrap();
    let target_main = targets.iter().find(|t| t.name == "main").unwrap();
    let camera_data = CameraUniform {
        view_proj: Mat4::IDENTITY,
        //view_proj: Mat4::look_at_rh(
        //    Vec3::new(3.0, 3.0, 3.0), // eye
        //    Vec3::ZERO,               // look_at
        //    Vec3::Y,                  // up
        //) * Mat4::IDENTITY, // model = identity
    };

    let bytes = unsafe {
        std::slice::from_raw_parts(
            &camera_data as *const _ as *const u8,
            std::mem::size_of::<CameraUniform>(),
        )
    };
    let camera_buffer = ctx
        .make_buffer(&BufferInfo {
            debug_name: "camera",
            byte_size: std::mem::size_of::<CameraUniform>() as u32,
            visibility: MemoryVisibility::CpuAndGpu,
            usage: BufferUsage::UNIFORM,
            initial_data: Some(bytes),
        })
        .unwrap();

    // -- Shader Stubs --
    let vert_shader = PipelineShaderInfo {
        stage: ShaderType::Vertex,
        spirv: include_spirv!(
            "assets/shaders/shadows.vert",
            vert
        ),
        specialization: &[],
    };

    let frag_shadow = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: include_spirv!(
            "assets/shaders/shadow_pass.frag",
            frag
        ),
        specialization: &[],
    };

    // Replace frag_lit with this updated shader
    let frag_lit = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: include_spirv!(
            "assets/shaders/shadows_lit.frag",
            frag
        ),
        specialization: &[],
    };
    let _vertex_info = VertexDescriptionInfo {
        entries: &[VertexEntryInfo {
            format: ShaderPrimitiveType::Vec3,
            location: 0,
            offset: 0,
        }],
        stride: 12,
        rate: VertexRate::Vertex,
    };

    // Add after creating the render pass and targets
    let shadow_depth_view = targets[0]
        .depth
        .as_ref()
        .expect("Missing shadow depth attachment")
        .attachment
        .img;

    // Add bind group layout for the depth sampler
    let lighting_bg_layout = ctx
        .make_bind_group_layout(&BindGroupLayoutInfo {
            debug_name: "lighting_depth_bg",
            shaders: &[
                ShaderInfo {
                    shader_type: ShaderType::Vertex,
                    variables: &[BindGroupVariable {
                        var_type: BindGroupVariableType::Uniform,
                        binding: 0,
                        count: 1,
                    }],
                },
                ShaderInfo {
                    shader_type: ShaderType::Fragment,
                    variables: &[BindGroupVariable {
                        var_type: BindGroupVariableType::SampledImage,
                        binding: 1,
                        count: 1,
                    }],
                },
            ],
        })
        .unwrap();
    let sampler = ctx.make_sampler(&Default::default()).unwrap();
    let camera_bind_group = ctx
        .make_bind_group(&BindGroupInfo {
            debug_name: "camera",
            layout: lighting_bg_layout,
            bindings: &[
                BindingInfo {
                    binding: 0,
                    resource: ShaderResource::Buffer(camera_buffer),
                },
                BindingInfo {
                    resource: ShaderResource::SampledImage(shadow_depth_view, sampler),
                    binding: 1,
                },
            ],
            set: 0,
        })
        .unwrap();

    // Inject layout into lighting pipeline layout
    let main_pipeline_layout = ctx
        .make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
            debug_name: "main_layout",
            vertex_info: VertexDescriptionInfo {
                entries: &[
                    VertexEntryInfo {
                        format: ShaderPrimitiveType::Vec3,
                        location: 0,
                        offset: 0,
                    },
                    VertexEntryInfo {
                        format: ShaderPrimitiveType::Vec3,
                        location: 1,
                        offset: 12,
                    },
                ],
                stride: std::mem::size_of::<Vertex>(),
                rate: VertexRate::Vertex,
            },
            bg_layouts: [Some(lighting_bg_layout), None, None, None],
            shaders: &[vert_shader.clone(), frag_lit.clone()],
            details: GraphicsPipelineDetails {
                depth_test: Some(DepthInfo {
                    should_test: true,
                    should_write: true,
                }),
                ..Default::default()
            },
        })
        .unwrap();
    // -- Pipelines --
    let shadow_pipeline_layout = ctx
        .make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
            debug_name: "shadow_pipeline",
            vertex_info: VertexDescriptionInfo {
                entries: &[
                    VertexEntryInfo {
                        format: ShaderPrimitiveType::Vec3,
                        location: 0,
                        offset: 0,
                    },
                    VertexEntryInfo {
                        format: ShaderPrimitiveType::Vec3,
                        location: 1,
                        offset: 12,
                    },
                ],
                stride: std::mem::size_of::<Vertex>(),
                rate: VertexRate::Vertex,
            },
            bg_layouts: [Some(lighting_bg_layout), None, None, None],
            shaders: &[vert_shader, frag_shadow],
            details: Default::default(),
        })
        .unwrap();

    let shadow_pipeline = ctx
        .make_graphics_pipeline(&GraphicsPipelineInfo {
            debug_name: "shadow_pipeline",
            layout: shadow_pipeline_layout,
            render_pass,
            subpass_id: 0,
        })
        .unwrap();

    let main_pipeline = ctx
        .make_graphics_pipeline(&GraphicsPipelineInfo {
            debug_name: "main_pipeline",
            layout: main_pipeline_layout,
            render_pass,
            subpass_id: 1,
        })
        .unwrap();

    // -- Winit and Render Loop --
    let mut display = ctx.make_display(&Default::default()).unwrap();
    let mut framed = FramedCommandList::new(ctx, "csm_frame", 2);
    let semaphores = ctx.make_semaphores(2).unwrap();

    println!("Starting render loop...");

    'running: loop {
        let mut should_exit = false;
        {
            let event_loop = display.winit_event_loop();
            event_loop.run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Exit;
                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        WindowEvent::CloseRequested |
                        WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), state: ElementState::Pressed, .. }, .. } => {
                            println!("Shutting down...");
                            should_exit = true;
                        }
                        _ => {}
                    }
                }
            });
        }
        if should_exit {
            break 'running;
        }

        let (image, acquire_sem, _, _) = ctx.acquire_new_image(&mut display).unwrap();

        framed.record(|cmd| {
            for (pipeline, target) in [(shadow_pipeline, &targets[0]), (main_pipeline, target_main)]
            {
                cmd.begin_drawing(&DrawBegin {
                    pipeline,
                    viewport,
                    attachments: &attachments
                        .attachments
                        .iter()
                        .map(|a| a.attachment.clone())
                        .collect::<Vec<_>>(),
                })
                .unwrap();

                cmd.draw_indexed(DrawIndexed {
                    vertices: vertex_buffer,
                    indices: index_buffer,
                    index_count: CUBE_INDICES.len() as u32,
                    bind_groups: [Some(camera_bind_group), None, None, None],
                    ..Default::default()
                });

                if target.name != "main" {
                    cmd.next_subpass().unwrap();
                }
            }

            cmd.end_drawing().unwrap();
            cmd.blit_image(ImageBlit {
                src: target_main.colors[0].attachment.img,
                dst: image,
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
