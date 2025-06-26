mod drawable;
pub use drawable::*;
mod time_stats;
pub use time_stats::*;

use crate::material::{BindlessLights, LightDesc, PSOBindGroupResources, PSO};
use crate::render_pass::*;
use crate::utils::{ResourceBinding, ResourceManager};
use crate::text::{FontRegistry, TextRenderable};
use dashi::utils::*;
use dashi::*;
use glam::Mat4;
use std::collections::HashMap;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::platform::run_return::EventLoopExtRunReturn;
/// Render pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderStage {
    Opaque,
    Text,
    // Extend as needed...
}

pub struct PipelineEntry {
    pub stage: RenderStage,
    pub pipeline: Handle<GraphicsPipeline>,
}

pub struct Renderer {
    ctx: *mut Context,
    display: Display,
    render_pass: Handle<RenderPass>,
    targets: Vec<RenderTarget>,
    stage_pipelines: HashMap<RenderStage, (PSO, [Option<PSOBindGroupResources>; 4])>,
    pipelines: HashMap<String, (PSO, [Option<PSOBindGroupResources>; 4])>,
    material_pipelines: HashMap<String, (PSO, [Option<PSOBindGroupResources>; 4])>,
    skeletal_pipeline: Option<(PSO, [Option<PSOBindGroupResources>; 4])>,
    resource_manager: ResourceManager,
    fonts: FontRegistry,
    lights: BindlessLights,
    drawables: Vec<(StaticMesh, Option<DynamicBuffer>)>,
    text_drawables: Vec<Box<dyn TextRenderable>>, 
    skeletal_meshes: Vec<(SkeletalMesh, Vec<SkeletalInstance>)>,
    command_list: FramedCommandList,
    semaphores: Vec<Handle<Semaphore>>,
    /// Tracks frame timing statistics for the renderer.
    time_stats: TimeStats,
    /// Handle to the GPU uniform buffer storing [`TimeStats`] data.
    time_buffer: Option<Handle<Buffer>>,
    clear_color: [f32; 4],
    clear_depth: f32,
    width: u32,
    height: u32,
}

impl Renderer {
    fn get_ctx(&mut self) -> &'static mut Context {
        unsafe { &mut *self.ctx }
    }

    pub fn with_render_pass(
        width: u32,
        height: u32,
        ctx: &mut Context,
        builder: RenderPassBuilder,
    ) -> Result<Self, GPUError> {
        let clear_color = [0.1, 0.2, 0.3, 1.0];
        let clear_depth = 1.0_f32;

        let ptr: *mut Context = ctx;
        let mut ctx: &mut Context = unsafe { &mut *ptr };
        let display = ctx.make_display(&DisplayInfo {
            window: WindowInfo {
                title: "KOJI-Renderer".to_string(),
                size: [width, height],
                ..Default::default()
            },
            ..Default::default()
        })?;

        let builder = builder.extent([width, height]);
        let (render_pass, targets, _attachments) = builder.build_with_images(&mut ctx)?;

        assert!(render_pass.valid());

        let command_list = FramedCommandList::new(&mut ctx, "RendererCmdList", 2);
        let semaphores = ctx.make_semaphores(2)?;

        let mut resource_manager = ResourceManager::new(&mut ctx, 4096)?;
        let lights = BindlessLights::new();
        lights.register(&mut resource_manager);
        resource_manager.register_time_buffers(&mut ctx);
        let time_buffer = match resource_manager.get("time") {
            Some(ResourceBinding::Uniform(h)) => Some(*h),
            _ => None,
        };

        let mut renderer = Self {
            ctx,
            display,
            render_pass,
            targets,
            stage_pipelines: HashMap::new(),
            pipelines: HashMap::new(),
            material_pipelines: HashMap::new(),
            skeletal_pipeline: None,
            drawables: Vec::new(),
            text_drawables: Vec::new(),
            skeletal_meshes: Vec::new(),
            resource_manager,
            fonts: FontRegistry::new(),
            lights,
            command_list,
            semaphores,
            time_stats: TimeStats::new(),
            time_buffer,
            width,
            height,
            clear_color,
            clear_depth,
        };

        // Initialize attachment clear values
        for target in &mut renderer.targets {
            for att in &mut target.colors {
                att.attachment.clear = ClearValue::Color(renderer.clear_color);
            }
            if let Some(depth) = &mut target.depth {
                depth.attachment.clear = ClearValue::DepthStencil {
                    depth: renderer.clear_depth,
                    stencil: 0,
                };
            }
        }

        Ok(renderer)
    }

    pub fn new(width: u32, height: u32, _title: &str, ctx: &mut Context) -> Result<Self, GPUError> {
        let builder = RenderPassBuilder::new()
            .debug_name("MainPass")
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
            .subpass("main", &["color"], &[] as &[&str]);

        Self::with_render_pass(width, height, ctx, builder)
    }

    pub fn with_render_pass_yaml(
        width: u32,
        height: u32,
        ctx: &mut Context,
        path: &str,
    ) -> Result<Self, GPUError> {
        let builder =
            RenderPassBuilder::from_yaml_file(path).map_err(|_| GPUError::LibraryError())?;
        Self::with_render_pass(width, height, ctx, builder)
    }
    pub fn render_pass(&self) -> Handle<RenderPass> {
        self.render_pass
    }

    pub fn set_clear_color(&mut self, color: [f32; 4]) {
        self.clear_color = color;
        for target in &mut self.targets {
            for att in &mut target.colors {
                att.attachment.clear = ClearValue::Color(color);
            }
        }
    }

    pub fn set_clear_depth(&mut self, depth: f32) {
        self.clear_depth = depth;
        for target in &mut self.targets {
            if let Some(ref mut att) = target.depth {
                att.attachment.clear = ClearValue::DepthStencil { depth, stencil: 0 };
            }
        }
    }

    /// Access timing statistics updated each frame.
    ///
    /// The returned [`TimeStats`] contains the elapsed and delta times in
    /// seconds. These values are also written to the `time` uniform buffer if
    /// available.
    pub fn time_stats(&self) -> &TimeStats {
        &self.time_stats
    }

    pub fn register_pso(
        &mut self,
        stage: RenderStage,
        pso: PSO,
        bind_group_resources: [Option<PSOBindGroupResources>; 4],
    ) {
        self.stage_pipelines
            .insert(stage, (pso, bind_group_resources));
    }

    pub fn register_pipeline_for_pass(
        &mut self,
        pass: &str,
        pso: PSO,
        bind_group_resources: [Option<PSOBindGroupResources>; 4],
    ) {
        self.pipelines
            .insert(pass.to_string(), (pso, bind_group_resources));
    }

    pub fn register_skeletal_pso(
        &mut self,
        pso: PSO,
        bind_group_resources: [Option<PSOBindGroupResources>; 4],
    ) {
        self.skeletal_pipeline = Some((pso, bind_group_resources));
    }

    pub fn register_material_pipeline(
        &mut self,
        material_id: &str,
        pso: PSO,
        bind_group_resources: [Option<PSOBindGroupResources>; 4],
    ) {
        self.material_pipelines
            .insert(material_id.to_string(), (pso, bind_group_resources));
    }

    pub fn register_static_mesh(
        &mut self,
        mut mesh: StaticMesh,
        dynamic_buffers: Option<DynamicBuffer>,
        material_id: String,
    ) {
        mesh.material_id = material_id;
        mesh.upload(self.get_ctx())
            .expect("Failed to upload mesh to GPU");
        self.drawables.push((mesh, dynamic_buffers));
    }

    pub fn register_text_mesh<T: TextRenderable + 'static>(&mut self, mesh: T) {
        self.text_drawables.push(Box::new(mesh));
    }

    pub fn update_text_mesh<T: TextRenderable + 'static>(&mut self, idx: usize, mesh: T) {
        if let Some(slot) = self.text_drawables.get_mut(idx) {
            *slot = Box::new(mesh);
        }
    }

    /// Upload a skeletal mesh and its instances.
    pub fn register_skeletal_mesh(
        &mut self,
        mut mesh: SkeletalMesh,
        instances: Vec<SkeletalInstance>,
        material_id: String,
    ) {
        mesh.material_id = material_id;
        mesh.upload(self.get_ctx())
            .expect("Failed to upload skeletal mesh to GPU");
        for inst in &instances {
            self.resource_manager
                .register_storage("bone_buf", inst.bone_buffer);
        }
        self.skeletal_meshes.push((mesh, instances));
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

    pub fn fonts(&self) -> &FontRegistry {
        &self.fonts
    }

    pub fn fonts_mut(&mut self) -> &mut FontRegistry {
        &mut self.fonts
    }

    /// Main render pass. The provided callback receives all window events as well
    /// as a final [`Event::MainEventsCleared`] each frame so the caller can
    /// update and draw.
    pub fn render_loop<F>(&mut self, mut draw_fn: F)
    where
        for<'a> F: FnMut(&mut Renderer, Event<'a, ()>),
    {
        'running: loop {
            let mut should_exit = false;
            let mut events: Vec<Event<'static, ()>> = Vec::new();
            {
                let event_loop = self.display.winit_event_loop();
                event_loop.run_return(|event, _, control_flow| {
                    *control_flow = ControlFlow::Exit;
                    if let Event::WindowEvent {
                        event: ref win_event,
                        ..
                    } = event
                    {
                        if matches!(
                            win_event,
                            WindowEvent::CloseRequested
                                | WindowEvent::KeyboardInput {
                                    input: KeyboardInput {
                                        virtual_keycode: Some(VirtualKeyCode::Escape),
                                        state: ElementState::Pressed,
                                        ..
                                    },
                                    ..
                                }
                        ) {
                            should_exit = true;
                        }
                    }
                    if let Some(evt) = event.to_static() {
                        events.push(evt);
                    }
                });
            }
            for event in events {
                draw_fn(self, event);
            }
            if should_exit {
                break 'running;
            }
            draw_fn(self, Event::MainEventsCleared);
            self.present_frame().unwrap();
        }
    }
    pub fn update_static_mesh(&mut self, idx: usize, vertices: &[Vertex]) {
        if let Some(mesh) = self.drawables.get_mut(idx) {
            mesh.0.vertices = vertices.to_vec();
            mesh.0
                .upload(unsafe { &mut *self.ctx })
                .expect("Failed to update mesh to GPU");
        }
    }

    /// Update bone matrices for a specific skeletal instance.
    pub fn update_skeletal_bones(&mut self, mesh_idx: usize, inst_idx: usize, matrices: &[Mat4]) {
        let ctx = self.get_ctx();
        if let Some((_mesh, instances)) = self.skeletal_meshes.get_mut(mesh_idx) {
            if let Some(inst) = instances.get_mut(inst_idx) {
                inst.animator.matrices.clone_from_slice(matrices);
                let _ = inst.update_gpu(ctx);
            }
        }
    }

    /// Advance an animation player and upload the new bone matrices.
    pub fn play_animation(&mut self, mesh_idx: usize, inst_idx: usize, dt: f32) {
        let ctx = self.get_ctx();
        if let Some((_mesh, instances)) = self.skeletal_meshes.get_mut(mesh_idx) {
            if let Some(inst) = instances.get_mut(inst_idx) {
                if let Some(player) = inst.player.as_mut() {
                    let local = player.advance(dt);
                    inst.animator.update_from_nodes(&local);
                    let _ = inst.update_gpu(ctx);
                }
            }
        }
    }

    /// Present one frame to display (for tests or non-interactive draw)
    pub fn present_frame(&mut self) -> Result<(), GPUError> {
        let ctx = self.get_ctx();
        self.time_stats.update();
        if let Some(buf) = self.time_buffer {
            let data = [self.time_stats.total_time, self.time_stats.delta_time];
            let slice: &mut [u8] = ctx.map_buffer_mut(buf)?;
            let bytes = bytemuck::bytes_of(&data);
            slice[..bytes.len()].copy_from_slice(bytes);
            ctx.unmap_buffer(buf)?;
        }
        self.lights.upload_all(ctx);
        let (img, acquire_sem, _img_idx, _) = ctx.acquire_new_image(&mut self.display)?;

        self.command_list.record(|list| {
            for target in &self.targets {
                // Collect attachments for drawing. Include depth if present.Add commentMore actions
                let attachments: Vec<Attachment> = {
                    let mut atts = target
                        .colors
                        .iter()
                        .map(|a| a.attachment)
                        .collect::<Vec<_>>();
                    if let Some(depth) = &target.depth {
                        atts.push(depth.attachment);
                    }
                    atts
                };

                let mut started = false;
                for (_idx, (mesh, _dynamic_buffers)) in self.drawables.iter().enumerate() {
                    let (pso, bind_groups) =
                        if let Some(entry) = self.material_pipelines.get(&mesh.material_id) {
                            entry
                        } else if let Some(entry) = self.pipelines.get(&target.name) {
                            entry
                        } else {
                            continue;
                        };
                    if !started {
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
                            pipeline: pso.pipeline,
                            attachments: &attachments,
                        })
                        .unwrap();
                        started = true;
                    } else {
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
                            pipeline: pso.pipeline,
                            attachments: &target
                                .colors
                                .iter()
                                .map(|a| a.attachment)
                                .collect::<Vec<_>>(),
                        })
                        .unwrap();
                    }

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
                if started {
                    list.end_drawing().unwrap();
                }

                if let Some((pso, bind_groups)) = self.stage_pipelines.get(&RenderStage::Text) {
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
                        pipeline: pso.pipeline,
                        attachments: &target
                            .colors
                            .iter()
                            .map(|a| a.attachment)
                            .collect::<Vec<_>>(),
                    })
                    .unwrap();

                    for mesh in &self.text_drawables {
                        let vb = mesh.vertex_buffer();
                        let ib = mesh.index_buffer();
                        let draw = if let Some(ib) = ib {
                            Command::DrawIndexed(DrawIndexed {
                                index_count: mesh.index_count() as u32,
                                instance_count: 1,
                                vertices: vb,
                                indices: ib,
                                bind_groups: [
                                    bind_groups[0].as_ref().map(|b| b.bind_group),
                                    bind_groups[1].as_ref().map(|b| b.bind_group),
                                    bind_groups[2].as_ref().map(|b| b.bind_group),
                                    bind_groups[3].as_ref().map(|b| b.bind_group),
                                ],
                                ..Default::default()
                            })
                        } else {
                            Command::Draw(Draw {
                                count: mesh.index_count() as u32,
                                instance_count: 1,
                                vertices: vb,
                                bind_groups: [
                                    bind_groups[0].as_ref().map(|b| b.bind_group),
                                    bind_groups[1].as_ref().map(|b| b.bind_group),
                                    bind_groups[2].as_ref().map(|b| b.bind_group),
                                    bind_groups[3].as_ref().map(|b| b.bind_group),
                                ],
                                ..Default::default()
                            })
                        };
                        list.append(draw);
                    }

                    list.end_drawing().unwrap();
                }

                for (mesh, instances) in &mut self.skeletal_meshes {
                    let (pso, bind_groups) =
                        if let Some(entry) = self.material_pipelines.get(&mesh.material_id) {
                            entry
                        } else if let Some(entry) = &self.skeletal_pipeline {
                            entry
                        } else {
                            continue;
                        };
                    let layout = pso.bind_group_layouts[0].expect("layout");
                    let mut started = false;
                    for inst in instances.iter_mut() {
                        inst.update_gpu(ctx).unwrap();
                        let inst_bg = ctx
                            .make_bind_group(&BindGroupInfo {
                                debug_name: "skel_instance_bg",
                                layout,
                                set: 0,
                                bindings: &[BindingInfo {
                                    binding: 0,
                                    resource: ShaderResource::StorageBuffer(inst.bone_buffer),
                                }],
                            })
                            .unwrap();

                        if !started {
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
                                pipeline: pso.pipeline,
                                attachments: &target
                                    .colors
                                    .iter()
                                    .map(|a| a.attachment)
                                    .collect::<Vec<_>>(),
                            })
                            .unwrap();
                            started = true;
                        } else {
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
                                pipeline: pso.pipeline,
                                attachments: &target
                                    .colors
                                    .iter()
                                    .map(|a| a.attachment)
                                    .collect::<Vec<_>>(),
                            })
                            .unwrap();
                        }

                        let vb = mesh.vertex_buffer.expect("Vertex buffer missing");
                        let ib = mesh.index_buffer;
                        let draw: dashi::Command = if let Some(ib) = ib {
                            Command::DrawIndexed(DrawIndexed {
                                index_count: mesh.index_count as u32,
                                instance_count: 1,
                                vertices: vb,
                                indices: ib,
                                bind_groups: [
                                    Some(inst_bg),
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
                                    Some(inst_bg),
                                    bind_groups[1].as_ref().map(|bgr| bgr.bind_group),
                                    bind_groups[2].as_ref().map(|bgr| bgr.bind_group),
                                    bind_groups[3].as_ref().map(|bgr| bgr.bind_group),
                                ],
                                ..Default::default()
                            })
                        };
                        list.append(draw);
                    }
                    if started {
                        list.end_drawing().unwrap();
                    }
                }

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

        self.get_ctx()
            .present_display(&self.display, &self.semaphores)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashi::gpu;
    use serial_test::serial;

    #[test]
    #[serial]
    #[cfg_attr(not(feature = "gpu_tests"), ignore)]
    fn set_clear_color_updates_attachments() {
        let device = gpu::DeviceSelector::new()
            .unwrap()
            .select(gpu::DeviceFilter::default().add_required_type(gpu::DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = gpu::Context::new(&gpu::ContextInfo { device }).unwrap();

        let mut renderer = Renderer::new(64, 64, "clr", &mut ctx).unwrap();
        renderer.set_clear_color([0.5, 0.5, 0.5, 1.0]);

        for target in &renderer.targets {
            for att in &target.colors {
                assert_eq!(
                    att.attachment.clear,
                    ClearValue::Color([0.5, 0.5, 0.5, 1.0])
                );
            }
        }
        ctx.destroy();
    }

    #[test]
    #[serial]
    #[cfg_attr(not(feature = "gpu_tests"), ignore)]
    fn set_clear_depth_updates_attachments() {
        let device = gpu::DeviceSelector::new()
            .unwrap()
            .select(gpu::DeviceFilter::default().add_required_type(gpu::DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = gpu::Context::new(&gpu::ContextInfo { device }).unwrap();

        let builder = RenderPassBuilder::new()
            .debug_name("clear_depth")
            .color_attachment("color", Format::RGBA8)
            .depth_attachment("depth", Format::D24S8)
            .subpass("main", ["color"], &[] as &[&str]);

        let mut renderer = Renderer::with_render_pass(64, 64, &mut ctx, builder).unwrap();
        renderer.set_clear_depth(0.25);

        for target in &renderer.targets {
            if let Some(ref depth) = target.depth {
                assert_eq!(
                    depth.attachment.clear,
                    ClearValue::DepthStencil {
                        depth: 0.25,
                        stencil: 0
                    }
                );
            }
        }

        ctx.destroy();
    }
}
