use bytemuck::*;
use dashi::utils::*;
use dashi::*;
use glam::*;
use inline_spirv::inline_spirv;
use koji::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::f32::consts::PI;
use std::time::Instant;

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
        spirv: inline_spirv!(
            r#"
#version 450
layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;
};

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;

layout(location = 0) out vec3 worldPos;
layout(location = 1) out vec3 normal;

void main() {
    mat4 model = mat4(1.0);
    vec4 world = model * vec4(inPosition, 1.0);
    worldPos = world.xyz;
    normal = mat3(model) * inNormal;
    gl_Position = view_proj * world;
}
            "#,
            vert
        ),
        specialization: &[],
    };

    let frag_shadow = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: inline_spirv!(
            r#"#version 450
            void main() {}"#,
            frag
        ),
        specialization: &[],
    };

    // Replace frag_lit with this updated shader
    let frag_lit = PipelineShaderInfo {
        stage: ShaderType::Fragment,
        spirv: inline_spirv!(
            r#"#version 450
        layout(location = 0) in vec3 vWorldPos;
        layout(location = 1) in vec3 vNormal;

        layout(set = 0, binding = 1) uniform sampler2DShadow shadowMap;

        layout(location = 0) out vec4 outColor;

        void main() {
            vec3 lightDir = normalize(vec3(-0.5, -1.0, -0.3));
            vec3 normal = normalize(vNormal);
            float NdotL = max(dot(normal, -lightDir), 0.0);

            // Sample the shadow map
            float shadow = texture(shadowMap, vec3(vWorldPos.xy * 0.5 + 0.5, vWorldPos.z));

            // Combine lighting with shadow factor
            vec3 litColor = vec3(1.0, 1.0, 0.8) * NdotL * shadow;
            outColor = vec4(litColor, 1.0);
        }
        "#,
            frag
        ),
        specialization: &[],
    };
    let vertex_info = VertexDescriptionInfo {
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

    // -- SDL and Render Loop --
    let mut display = ctx.make_display(&Default::default()).unwrap();
    let mut event_pump = ctx.get_sdl_ctx().event_pump().unwrap();
    let mut framed = FramedCommandList::new(ctx, "csm_frame", 2);
    let semaphores = ctx.make_semaphores(2).unwrap();

    println!("Starting render loop...");

    'running: loop {
        for e in event_pump.poll_iter() {
            if matches!(
                e,
                Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    }
            ) {
                println!("Shutting down...");
                break 'running;
            }
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
}

//// src/cascaded_shadows.rs
//use dashi::utils::*;
//use dashi::*;
//use inline_spirv::inline_spirv;
//use koji::*;
//use sdl2::event::Event;
//use sdl2::keyboard::Keycode;
//use std::time::Instant;
//
//pub fn run(ctx: &mut Context) {
//    const WIDTH: u32 = 640;
//    const HEIGHT: u32 = 480;
//    const NUM_CASCADES: usize = 4;
//
//    let vertices: [[f32; 2]; 3] = [[-0.5, -0.5], [0.0, 0.5], [0.5, -0.5]];
//    let vertex_buffer = ctx
//        .make_buffer(&BufferInfo {
//            debug_name: "shadow_triangle",
//            byte_size: (vertices.len() * std::mem::size_of::<[f32; 2]>()) as u32,
//            visibility: MemoryVisibility::Gpu,
//            usage: BufferUsage::VERTEX,
//            initial_data: unsafe { Some(vertices.align_to::<u8>().1) },
//        })
//        .unwrap();
//
//    let mut builder = RenderPassBuilder::new()
//        .debug_name("cascaded_shadow_pass")
//        .extent([WIDTH, HEIGHT])
//        .viewport(Viewport {
//            area: FRect2D {
//                w: WIDTH as f32,
//                h: HEIGHT as f32,
//                ..Default::default()
//            },
//            scissor: Rect2D {
//                w: WIDTH,
//                h: HEIGHT,
//                ..Default::default()
//            },
//            ..Default::default()
//        });
//
//    for i in 0..NUM_CASCADES {
//        builder = builder.color_attachment(format!("cascade{}", i), Format::RGBA8);
//    }
//
//    for i in 0..NUM_CASCADES {
//        let dep_name = if i == 0 {
//            "".to_string()
//        } else {
//            format!("cascade{}", i - 1)
//        };
//        let d = if i == 0 {
//            &[] as &[&str]
//        } else {
//            &[dep_name.as_str()]
//        };
//
//        builder = builder.subpass(format!("cascade{}", i), [format!("cascade{}", i)], None, d);
//    }
//
//    let (render_pass, targets, attachments) = builder.build_with_images(ctx).unwrap();
//
//    let vert = PipelineShaderInfo {
//        stage: ShaderType::Vertex,
//        spirv: inline_spirv!(
//            r#"#version 450
//            layout(location = 0) in vec2 inPosition;
//            void main() {
//                gl_Position = vec4(inPosition, 0.0, 1.0);
//            }"#,
//            vert
//        ),
//        specialization: &[],
//    };
//
//    let frag = PipelineShaderInfo {
//        stage: ShaderType::Fragment,
//        spirv: inline_spirv!(
//            r#"#version 450
//            layout(location = 0) out vec4 outColor;
//            void main() {
//                outColor = vec4(0.2, 0.2, 0.2, 1.0);
//            }"#,
//            frag
//        ),
//        specialization: &[],
//    };
//
//    let vertex_info = VertexDescriptionInfo {
//        entries: &[VertexEntryInfo {
//            format: ShaderPrimitiveType::Vec2,
//            location: 0,
//            offset: 0,
//        }],
//        stride: 8,
//        rate: VertexRate::Vertex,
//    };
//
//    let pipeline_layout = ctx
//        .make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
//            debug_name: "shadow_pipeline_layout",
//            vertex_info,
//            bg_layouts: [None, None, None, None],
//            shaders: &[vert, frag],
//            details: GraphicsPipelineDetails {
//                culling: CullMode::None,
//                ..Default::default()
//            },
//        })
//        .unwrap();
//
//    let pipelines: Vec<_> = (0..NUM_CASCADES)
//        .map(|i| {
//            ctx.make_graphics_pipeline(&GraphicsPipelineInfo {
//                layout: pipeline_layout,
//                render_pass,
//                subpass_id: i as u8,
//                debug_name: Box::leak(format!("shadow_pipeline_{}", i).into_boxed_str()),
//            })
//            .unwrap()
//        })
//        .collect();
//
//    let mut display = ctx.make_display(&Default::default()).unwrap();
//    let semaphores = ctx.make_semaphores(2).unwrap();
//    let mut framed = FramedCommandList::new(ctx, "cascaded", 2);
//    let mut event_pump = ctx.get_sdl_ctx().event_pump().unwrap();
//    let mut _timer = Instant::now();
//
//    'main: loop {
//        for e in event_pump.poll_iter() {
//            if matches!(
//                e,
//                Event::Quit { .. }
//                    | Event::KeyDown {
//                        keycode: Some(Keycode::Escape),
//                        ..
//                    }
//            ) {
//                break 'main;
//            }
//        }
//
//        let (img, acquire_sem, _, _) = ctx.acquire_new_image(&mut display).unwrap();
//        framed.record(|cmd| {
//            for (i, pipeline) in pipelines.iter().enumerate() {
//                let target = &targets[i];
//                cmd.begin_drawing(&DrawBegin {
//                    pipeline: *pipeline,
//                    viewport: Viewport {
//                        area: FRect2D {
//                            w: WIDTH as f32,
//                            h: HEIGHT as f32,
//                            ..Default::default()
//                        },
//                        scissor: Rect2D {
//                            w: WIDTH,
//                            h: HEIGHT,
//                            ..Default::default()
//                        },
//                        ..Default::default()
//                    },
//                    attachments: &attachments
//                        .attachments
//                        .iter()
//                        .map(|a| a.attachment.clone())
//                        .collect::<Vec<_>>(),
//                })
//                .unwrap();
//
//                cmd.append(Command::DrawCommand(Draw {
//                    vertices: vertex_buffer,
//                    count: 3,
//                    instance_count: 1,
//                    ..Default::default()
//                }));
//
//                if i < pipelines.len() - 1 {
//                    cmd.next_subpass().unwrap();
//                }
//            }
//
//            cmd.end_drawing().unwrap();
//            cmd.blit(ImageBlit {
//                src: targets.last().unwrap().colors[0].attachment.img,
//                dst: img,
//                filter: Filter::Nearest,
//                ..Default::default()
//            });
//        });
//
//        framed.submit(&SubmitInfo {
//            wait_sems: &[acquire_sem],
//            signal_sems: &semaphores,
//        });
//
//        ctx.present_display(&display, &semaphores).unwrap();
//    }
//}
//
//pub fn main() {
//    let device = DeviceSelector::new()
//        .unwrap()
//        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
//        .unwrap_or_default();
//    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
//    run(&mut ctx);
//}
