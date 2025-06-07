mod drawable;
pub use drawable::*;

use crate::material::{BindlessLights, LightDesc, PSOBindGroupResources, PSO};
use crate::utils::ResourceManager;
use dashi::utils::*;
use crate::render_pass::*;
use dashi::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::collections::HashMap;
/// Render pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderStage {
    Opaque,
    // Extend as needed...
}

pub struct PipelineEntry {
    pub stage: RenderStage,
    pub pipeline: Handle<GraphicsPipeline>,
}

pub struct Renderer {
    ctx: * mut Context,
    display: Display,
    event_pump: sdl2::EventPump,
    render_pass: Handle<RenderPass>,
    targets: Vec<RenderTarget>,
    pipelines: HashMap<RenderStage, (PSO, [Option<PSOBindGroupResources>; 4])>,
    resource_manager: ResourceManager,
    lights: BindlessLights,
    drawables: Vec<(StaticMesh, Option<DynamicBuffer>)>,
    command_list: FramedCommandList,
    semaphores: Vec<Handle<Semaphore>>,
    clear_color: [f32; 4],
    width: u32,
    height: u32,
}

impl Renderer {
    fn get_ctx(&mut self) -> &'static mut Context {
       unsafe{&mut *self.ctx}
    }

    pub fn new(width: u32, height: u32, _title: &str, ctx: &mut Context) -> Result<Self, GPUError> {
        let clear_color = [0.1, 0.2, 0.3, 1.0];

        let ptr: *mut Context =  ctx;
        let mut ctx: &mut Context =  unsafe{&mut *ptr};
        let display = ctx.make_display(&DisplayInfo {
            ..Default::default()
        })?;

        // Main pass: 1 color attachment, no depth
        let (render_pass, targets, _attachments) = RenderPassBuilder::new()
            .debug_name("MainPass")
            .extent([width, height])
            .viewport(Viewport {
                area: FRect2D {
                    w: width as f32,
                    h: height as f32,
                    ..Default::default()
                },
                scissor: Rect2D {
                    w: width,
                    h: height,
                    ..Default::default()
                },
                ..Default::default()
            })
            .color_attachment("color", Format::RGBA8)
            .subpass("main", &["color"], &[] as &[&str])
            .build_with_images(&mut ctx)?;

        assert!(render_pass.valid());
        let event_pump = ctx.get_sdl_ctx().event_pump().unwrap();

        let command_list = FramedCommandList::new(&mut ctx, "RendererCmdList", 2);
        let semaphores = ctx.make_semaphores(2)?;

        let mut resource_manager = ResourceManager::new(&mut ctx, 4096)?;
        let lights = BindlessLights::new();
        lights.register(&mut resource_manager);

        Ok(Self {
            ctx,
            display,
            event_pump,
            render_pass,
            targets,
            pipelines: HashMap::new(),
            drawables: Vec::new(),
            resource_manager,
            lights,
            command_list,
            semaphores,
            width,
            height,
            clear_color,
        })
    }
    pub fn render_pass(&self) -> Handle<RenderPass> {
        self.render_pass
    }

    pub fn set_clear_color(&mut self, color: [f32; 4]) {
        self.clear_color = color;
    }

    pub fn register_pso(
        &mut self,
        stage: RenderStage,
        pso: PSO,
        bind_group_resources: [Option<PSOBindGroupResources>; 4],
    ) {
        self.pipelines.insert(stage, (pso, bind_group_resources));
    }

    pub fn register_static_mesh(
        &mut self,
        mut mesh: StaticMesh,
        dynamic_buffers: Option<DynamicBuffer>,
    ) {
        mesh.upload(self.get_ctx())
            .expect("Failed to upload mesh to GPU");
        self.drawables.push((mesh, dynamic_buffers));
    }

    pub fn add_light(&mut self, light: LightDesc) -> u32 {
        let ctx = self.get_ctx();
        let res = &mut self.resource_manager;
        self.lights.add_light(ctx, res, light)
    }

    pub fn update_light(&mut self, index: usize, light: LightDesc) {
        let ctx = self.get_ctx();
        self.lights.update_light(ctx, index, light);
    }

    pub fn resources(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }

    /// Main render pass, returns false if quit requested
    pub fn render_loop<F: FnMut(&mut Renderer)>(&mut self, mut draw_fn: F) {
        'running: loop {
            for event in self.event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }
            draw_fn(self);
            self.present_frame().unwrap();
        }
    }
    pub fn update_static_mesh(&mut self, idx: usize, vertices: &[Vertex]) {
        if let Some(mesh) = self.drawables.get_mut(idx) {
            mesh.0.vertices = vertices.to_vec();
            mesh.0
                .upload(unsafe{&mut *self.ctx})
                .expect("Failed to update mesh to GPU");
        }
    }

    /// Present one frame to display (for tests or non-interactive draw)
    pub fn present_frame(&mut self) -> Result<(), GPUError> {
        let ctx = self.get_ctx();
        self.lights.upload_all(ctx);
        let (img, acquire_sem, _img_idx, _) = ctx.acquire_new_image(&mut self.display)?;

        self.command_list.record(|list| {
            for target in &self.targets {
                list.begin_drawing(&DrawBegin {
                    viewport: Viewport {
                        area: FRect2D {
                            w: self.width as f32,
                            h: self.height as f32,
                            ..Default::default()
                        },
                        scissor: Rect2D {
                            w: self.width,
                            h: self.height,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    pipeline: self.pipelines[&RenderStage::Opaque].0.pipeline,
                    attachments: &target
                        .colors
                        .iter()
                        .map(|a| a.attachment)
                        .collect::<Vec<_>>(),
                })
                .unwrap();
                let (_pso, bind_groups) = &self.pipelines[&RenderStage::Opaque];
                for (_idx, (mesh, _dynamic_buffers)) in self.drawables.iter().enumerate() {
                    let vb = mesh.vertex_buffer.expect("Vertex buffer missing");
                    let ib = mesh.index_buffer;
                    let draw: dashi::Command = if let Some(ib) = ib {
                        Command::DrawIndexed(DrawIndexed {
                            index_count: mesh.index_count as u32,
                            instance_count: 1,
                            vertices: vb,
                            indices: ib,
                            bind_groups: [
                                bind_groups[0].as_ref().map(|bgr| bgr.bind_group),
                                bind_groups[1].as_ref().map(|bgr| bgr.bind_group),
                                bind_groups[2].as_ref().map(|bgr| bgr.bind_group),
                                bind_groups[3].as_ref().map(|bgr| bgr.bind_group),
                            ],
                            ..Default::default()
                        })
                    } else {
                        Command::Draw(Draw {
                            count: mesh.index_count as u32,
                            instance_count: 1,
                            vertices: vb,
                            bind_groups: [
                                bind_groups[0].as_ref().map(|bgr| bgr.bind_group),
                                bind_groups[1].as_ref().map(|bgr| bgr.bind_group),
                                bind_groups[2].as_ref().map(|bgr| bgr.bind_group),
                                bind_groups[3].as_ref().map(|bgr| bgr.bind_group),
                            ],
                            ..Default::default()
                        })
                    };
                    list.append(draw);
                }

                list.end_drawing().unwrap();

                list.blit_image(ImageBlit {
                    src: target.colors[0].attachment.img,
                    dst: img,
                    filter: Filter::Nearest,
                    ..Default::default()
                });
            }
        });

        self.command_list.submit(&SubmitInfo {
            wait_sems: &[acquire_sem],
            signal_sems: &self.semaphores,
        });

        self.get_ctx().present_display(&self.display, &self.semaphores)?;
        Ok(())
    }
}
