mod drawable;
pub use drawable::*;
mod time_stats;
pub use time_stats::*;

use crate::canvas::CanvasBuilder;
use crate::material::{BindlessLights, LightDesc, PSOBindGroupResources, CPSO, PSO};
use crate::render_graph::{RenderGraph, RenderPassNode, ResourceDesc};
use crate::render_pass::*;
use crate::text::{FontRegistry, TextRenderable};
use crate::utils::{diff_rgba8, ResourceBinding, ResourceManager};
use dashi::utils::*;
use dashi::*;
use glam::Mat4;
use std::collections::HashMap;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::platform::run_return::EventLoopExtRunReturn;

mod draw_log {
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    pub static LOG: Lazy<Mutex<Vec<&'static str>>> = Lazy::new(|| Mutex::new(Vec::new()));

    pub fn log(event: &'static str) {
        LOG.lock().unwrap().push(event);
    }

    pub fn take() -> Vec<&'static str> {
        LOG.lock().unwrap().drain(..).collect()
    }
}

pub mod test_hooks {
    /// Retrieve and clear recorded draw events.
    pub fn take_draw_events() -> Vec<&'static str> {
        super::draw_log::take()
    }
}
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

#[derive(Clone)]
pub struct ComputeTask {
    pub id: String,
    pub groups: [u32; 3],
}

pub struct Renderer {
    ctx: *mut Context,
    display: Option<Display>,
    render_pass: Handle<RenderPass>,
    targets: Vec<RenderTarget>,
    canvases: Vec<crate::canvas::Canvas>,
    stage_pipelines: HashMap<RenderStage, (PSO, [Option<PSOBindGroupResources>; 4])>,
    pipelines: HashMap<String, (PSO, [Option<PSOBindGroupResources>; 4])>,
    material_pipelines: HashMap<String, (PSO, [Option<PSOBindGroupResources>; 4])>,
    skeletal_pipeline: Option<(PSO, [Option<PSOBindGroupResources>; 4])>,
    compute_pipelines: HashMap<String, (CPSO, [Option<PSOBindGroupResources>; 4])>,
    compute_queue: Vec<ComputeTask>,
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
    graph: crate::render_graph::RenderGraph,
    width: u32,
    height: u32,
}

impl Renderer {
    fn get_ctx(&mut self) -> &'static mut Context {
        unsafe { &mut *self.ctx }
    }

    fn with_graph_internal(
        width: u32,
        height: u32,
        ctx: &mut Context,
        graph: RenderGraph,
        headless: bool,
    ) -> Result<Self, GPUError> {
        let clear_color = [0.1, 0.2, 0.3, 1.0];
        let clear_depth = 1.0_f32;

        let ptr: *mut Context = ctx;
        let mut ctx: &mut Context = unsafe { &mut *ptr };
        let display = if headless {
            None
        } else {
            Some(ctx.make_display(&DisplayInfo {
                window: WindowInfo {
                    title: "KOJI-Renderer".to_string(),
                    size: [width, height],
                    ..Default::default()
                },
                ..Default::default()
            })?)
        };

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
            .depth_attachment("depth", Format::D24S8)
            .subpass("main", ["color"], &[] as &[&str])
            .extent([width, height]);
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
            canvases: Vec::new(),
            stage_pipelines: HashMap::new(),
            pipelines: HashMap::new(),
            material_pipelines: HashMap::new(),
            skeletal_pipeline: None,
            compute_pipelines: HashMap::new(),
            compute_queue: Vec::new(),
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
            graph: crate::render_graph::RenderGraph::new(),
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

        let mut canvases = graph.canvases();
        for canvas in &mut canvases {
            for att in &mut canvas.target_mut().colors {
                att.attachment.clear = ClearValue::Color(renderer.clear_color);
            }
            if let Some(depth) = &mut canvas.target_mut().depth {
                depth.attachment.clear = ClearValue::DepthStencil {
                    depth: renderer.clear_depth,
                    stencil: 0,
                };
            }
        }

        renderer.canvases = canvases;
        renderer.graph = graph;

        Ok(renderer)
    }

    pub fn with_canvas(
        width: u32,
        height: u32,
        ctx: &mut Context,
        canvas: crate::canvas::Canvas,
    ) -> Result<Self, GPUError> {
        let outputs = canvas
            .target()
            .colors
            .iter()
            .map(|a| ResourceDesc {
                name: a.name.clone(),
                format: a.format,
            })
            .collect();
        let node = RenderPassNode::new("main", canvas.render_pass(), Vec::new(), outputs);
        let mut graph = RenderGraph::new();
        graph.add_node(node);
        graph.add_canvas(&canvas);

        Self::with_graph_internal(width, height, ctx, graph, false)
    }

    pub fn with_canvas_headless(
        width: u32,
        height: u32,
        ctx: &mut Context,
        canvas: crate::canvas::Canvas,
    ) -> Result<Self, GPUError> {
        let outputs = canvas
            .target()
            .colors
            .iter()
            .map(|a| ResourceDesc {
                name: a.name.clone(),
                format: a.format,
            })
            .collect();
        let node = RenderPassNode::new("main", canvas.render_pass(), Vec::new(), outputs);
        let mut graph = RenderGraph::new();
        graph.add_node(node);
        graph.add_canvas(&canvas);

        Self::with_graph_internal(width, height, ctx, graph, true)
    }

    pub fn with_graph(
        width: u32,
        height: u32,
        ctx: &mut Context,
        graph: RenderGraph,
    ) -> Result<Self, GPUError> {
        Self::with_graph_internal(width, height, ctx, graph, false)
    }

    pub fn with_graph_headless(
        width: u32,
        height: u32,
        ctx: &mut Context,
        graph: RenderGraph,
    ) -> Result<Self, GPUError> {
        Self::with_graph_internal(width, height, ctx, graph, true)
    }

    pub fn new(width: u32, height: u32, _title: &str, ctx: &mut Context) -> Result<Self, GPUError> {
        let canvas = CanvasBuilder::new()
            .extent([width, height])
            .color_attachment("color", Format::RGBA8)
            .build(ctx)?;

        Self::with_canvas(width, height, ctx, canvas)
    }

    pub fn new_headless(
        width: u32,
        height: u32,
        _title: &str,
        ctx: &mut Context,
    ) -> Result<Self, GPUError> {
        let canvas = CanvasBuilder::new()
            .extent([width, height])
            .color_attachment("color", Format::RGBA8)
            .build(ctx)?;

        Self::with_canvas_headless(width, height, ctx, canvas)
    }

    pub fn render_pass(&self) -> Handle<RenderPass> {
        self.render_pass
    }

    pub fn graph(&self) -> &crate::render_graph::RenderGraph {
        &self.graph
    }

    pub fn set_clear_color(&mut self, color: [f32; 4]) {
        self.clear_color = color;
        for target in &mut self.targets {
            for att in &mut target.colors {
                att.attachment.clear = ClearValue::Color(color);
            }
        }
        for canvas in &mut self.canvases {
            for att in &mut canvas.target_mut().colors {
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
        for canvas in &mut self.canvases {
            if let Some(ref mut att) = canvas.target_mut().depth {
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

    pub fn register_compute_pipeline(
        &mut self,
        id: &str,
        pso: CPSO,
        bgr: [Option<PSOBindGroupResources>; 4],
    ) {
        self.compute_pipelines.insert(id.to_string(), (pso, bgr));
    }

    pub fn queue_compute(&mut self, id: &str, groups: [u32; 3]) {
        self.compute_queue.push(ComputeTask {
            id: id.to_string(),
            groups,
        });
    }

    pub fn add_canvas(&mut self, canvas: crate::canvas::Canvas) {
        self.canvases.push(canvas);
    }

    pub fn canvas(&self, index: usize) -> Option<&crate::canvas::Canvas> {
        self.canvases.get(index)
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
        if self.display.is_none() {
            draw_fn(self, Event::MainEventsCleared);
            self.present_frame().unwrap();
            return;
        }

        'running: loop {
            let mut should_exit = false;
            let mut events: Vec<Event<'static, ()>> = Vec::new();
            {
                let event_loop = self.display.as_mut().unwrap().winit_event_loop();
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

    /// Helper to build [`DrawBegin`] for a target/pipeline pair.
    fn prepare_draw_begin<'a>(
        width: u32,
        height: u32,
        target: &'a RenderTarget,
        pipeline: Handle<GraphicsPipeline>,
        attachments: &'a mut Vec<Attachment>,
        include_depth: bool,
    ) -> DrawBegin<'a> {
        attachments.clear();
        attachments.extend(target.colors.iter().map(|a| a.attachment));
        if include_depth {
            if let Some(depth) = &target.depth {
                attachments.push(depth.attachment);
            }
        }

        DrawBegin {
            viewport: Viewport {
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
            },
            pipeline,
            attachments,
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
        let (img, acquire_sem) = if let Some(display) = self.display.as_mut() {
            let (img, sem, _img_idx, _) = ctx.acquire_new_image(display)?;
            (Some(img), Some(sem))
        } else {
            (None, None)
        };

        let width = self.width;
        let height = self.height;
        let use_canvas_blit = self.display.is_some()
            && self.targets.is_empty()
            && !self.canvases.is_empty();

        self.command_list.record(|list| {
            for task in self.compute_queue.drain(..) {
                if let Some((pso, bgr)) = self.compute_pipelines.get(&task.id) {
                    list.dispatch_compute(Dispatch {
                        compute: pso.pipeline,
                        workgroup_size: task.groups,
                        bind_groups: [
                            bgr[0].as_ref().map(|r| r.bind_group),
                            bgr[1].as_ref().map(|r| r.bind_group),
                            bgr[2].as_ref().map(|r| r.bind_group),
                            bgr[3].as_ref().map(|r| r.bind_group),
                        ],
                        ..Default::default()
                    });
                }
            }
            let canvas_len = self.canvases.len();
            for (idx, target) in self
                .canvases
                .iter()
                .map(|c| c.target())
                .chain(self.targets.iter())
                .enumerate()
            {
                let mut attachments = Vec::new();
                let mut current_pipeline: Option<Handle<GraphicsPipeline>> = None;
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

                    if Some(pso.pipeline) != current_pipeline {
                        if !started {
                            let draw_begin = Self::prepare_draw_begin(
                                width,
                                height,
                                &target,
                                pso.pipeline,
                                &mut attachments,
                                true,
                            );
                            list.begin_drawing(&draw_begin).unwrap();
                            list.set_viewport(draw_begin.viewport);
                            list.set_scissor(draw_begin.viewport.scissor);
                            #[cfg(test)]
                            draw_log::log("begin_static");
                            started = true;
                        } else {
                            let draw_begin = Self::prepare_draw_begin(
                                width,
                                height,
                                &target,
                                pso.pipeline,
                                &mut attachments,
                                true,
                            );
                            list.begin_drawing(&draw_begin).unwrap();
                            list.set_viewport(draw_begin.viewport);
                            list.set_scissor(draw_begin.viewport.scissor);
                            #[cfg(test)]
                            draw_log::log("bind_static");
                        }
                        current_pipeline = Some(pso.pipeline);
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
                    #[cfg(test)]
                    draw_log::log("end_static");
                }

                if !self.text_drawables.is_empty() {
                    if let Some((pso, bind_groups)) = self.stage_pipelines.get(&RenderStage::Text) {
                        let mut attachments = Vec::new();
                        let draw_begin = Self::prepare_draw_begin(
                            width,
                            height,
                            &target,
                            pso.pipeline,
                            &mut attachments,
                            false,
                        );
                        list.begin_drawing(&draw_begin).unwrap();
                        list.set_viewport(draw_begin.viewport);
                        list.set_scissor(draw_begin.viewport.scissor);

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
                    let mut attachments = Vec::new();
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
                            let draw_begin = Self::prepare_draw_begin(
                                width,
                                height,
                                &target,
                                pso.pipeline,
                                &mut attachments,
                                false,
                            );
                            list.begin_drawing(&draw_begin).unwrap();
                            list.set_viewport(draw_begin.viewport);
                            list.set_scissor(draw_begin.viewport.scissor);
                            #[cfg(test)]
                            draw_log::log("begin_skeletal");
                            started = true;
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
                        #[cfg(test)]
                        draw_log::log("end_skeletal");
                    }
                }

                if let Some(img) = img {
                    if idx >= canvas_len {
                        list.blit_image(ImageBlit {
                            src: target.colors[0].attachment.img,
                            dst: img,
                            filter: Filter::Nearest,
                            ..Default::default()
                        });
                    }
                }
            }

            if use_canvas_blit {
                if let Some(img) = img {
                    let canvas = &self.canvases[0];
                    let tgt = canvas.target();
                    list.blit_image(ImageBlit {
                        src: tgt.colors[0].attachment.img,
                        dst: img,
                        filter: Filter::Nearest,
                        ..Default::default()
                    });
                }
            }
        });

        let mut wait_sems = Vec::new();
        if let Some(sem) = acquire_sem {
            wait_sems.push(sem);
        }
        self.command_list.submit(&SubmitInfo {
            wait_sems: &wait_sems,
            signal_sems: &self.semaphores,
        });

        if let Some(display) = self.display.as_ref() {
            ctx.present_display(display, &self.semaphores)?;
        }
        Ok(())
    }

    /// Read back the specified color attachment into a CPU-accessible RGBA8 buffer.
    pub fn read_color_target(&mut self, name: &str) -> Vec<u8> {
        let ctx = self.get_ctx();
        let attachment = self
            .canvases
            .iter()
            .map(|c| c.target())
            .chain(self.targets.iter())
            .find_map(|t| t.colors.iter().find(|a| a.name == name))
            .expect("color attachment not found");

        let view = attachment.attachment.img;
        let byte_size = (self.width * self.height * 4) as u32;
        let buffer = ctx
            .make_buffer(&BufferInfo {
                debug_name: "readback",
                byte_size,
                visibility: MemoryVisibility::CpuAndGpu,
                ..Default::default()
            })
            .expect("readback buffer");

        let mut list = ctx
            .begin_command_list(&CommandListInfo {
                debug_name: "readback_copy",
                ..Default::default()
            })
            .expect("command list");
        list.copy_image_to_buffer(ImageBufferCopy {
            src: view,
            dst: buffer,
            dst_offset: 0,
        });
        let fence = ctx.submit(&mut list, &SubmitInfo::default()).expect("submit");
        ctx.wait(fence).expect("wait");

        let data = ctx.map_buffer::<u8>(buffer).expect("map").to_vec();
        ctx.unmap_buffer(buffer).expect("unmap");

        ctx.destroy_cmd_list(list);
        ctx.destroy_buffer(buffer);
        ctx.destroy_fence(fence);

        data
    }

    /// Compute the mean absolute per-channel difference between the current
    /// `"color"` attachment and a reference frame.
    ///
    /// The reference slice must contain `width * height * 4` RGBA8 bytes. This
    /// is primarily intended for headless testing where rendered frames are
    /// compared against known-good outputs. The returned value is normalized to
    /// `0.0..=1.0`.
    pub fn frame_difference(&mut self, reference: &[u8]) -> f32 {
        let current = self.read_color_target("color");
        diff_rgba8(&current, reference)
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
    #[ignore]
    fn set_clear_color_updates_attachments() {
        let device = gpu::DeviceSelector::new()
            .unwrap()
            .select(gpu::DeviceFilter::default().add_required_type(gpu::DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = gpu::Context::new(&gpu::ContextInfo { device }).unwrap();
        let canvas = CanvasBuilder::new()
            .extent([64, 64])
            .color_attachment("color", Format::RGBA8)
            .build(&mut ctx)
            .unwrap();

        let mut renderer = Renderer::with_canvas(64, 64, &mut ctx, canvas).unwrap();
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
    #[ignore]
    fn set_clear_depth_updates_attachments() {
        let device = gpu::DeviceSelector::new()
            .unwrap()
            .select(gpu::DeviceFilter::default().add_required_type(gpu::DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = gpu::Context::new(&gpu::ContextInfo { device }).unwrap();

        let canvas = CanvasBuilder::new()
            .extent([64, 64])
            .color_attachment("color", Format::RGBA8)
            .build(&mut ctx)
            .unwrap();

        let mut renderer = Renderer::with_canvas(64, 64, &mut ctx, canvas).unwrap();
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
