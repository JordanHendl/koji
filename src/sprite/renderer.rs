use crate::material::{pipeline_builder::PipelineBuilder, PSO, PSOBindGroupResources};
use crate::utils::*;
use crate::render_pass::*;
use crate::sprite::{Sprite, SpriteVertex};
use dashi::utils::*;
use dashi::*;

pub struct SpriteRenderer {
    ctx: *mut Context,
    display: Display,
    render_pass: Handle<RenderPass>,
    targets: Vec<RenderTarget>,
    command_list: FramedCommandList,
    semaphores: Vec<Handle<Semaphore>>,
    pso: PSO,
    bind_groups: [Option<PSOBindGroupResources>; 4],
    resource_manager: ResourceManager,
    sprites: Vec<Sprite>,
    width: u32,
    height: u32,
}

impl SpriteRenderer {
    fn get_ctx(&mut self) -> &mut Context {
        unsafe { &mut *self.ctx }
    }

    pub fn new(width: u32, height: u32, ctx: &mut Context) -> Result<Self, GPUError> {
        let ptr = ctx as *mut Context;
        let display = ctx.make_display(&DisplayInfo::default())?;

        let (render_pass, targets, _attachments) = RenderPassBuilder::new()
            .debug_name("SpritePass")
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
            .build_with_images(ctx)?;

        let command_list = FramedCommandList::new(ctx, "SpriteCmdList", 2);
        let semaphores = ctx.make_semaphores(2)?;

        let vert: &[u32] = inline_spirv::include_spirv!("shaders/sprite.vert", vert, glsl);
        let frag: &[u32] = inline_spirv::include_spirv!("shaders/sprite.frag", frag, glsl);

        let mut pso = PipelineBuilder::new(ctx, "sprite_pipeline")
            .vertex_shader(vert)
            .fragment_shader(frag)
            .render_pass(render_pass, 0)
            .build();

        let resource_manager = ResourceManager::new(ctx, 1024)?;
        let bind_groups = pso.create_bind_groups(&resource_manager);

        Ok(Self {
            ctx: ptr,
            display,
            render_pass,
            targets,
            command_list,
            semaphores,
            pso,
            bind_groups,
            resource_manager,
            sprites: Vec::new(),
            width,
            height,
        })
    }

    pub fn resources(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }

    pub fn update_bind_groups(&mut self) {
        self.bind_groups = self.pso.create_bind_groups(&self.resource_manager);
    }

    pub fn register_sprite(&mut self, mut sprite: Sprite) {
        sprite.upload(self.get_ctx()).expect("upload sprite");
        self.sprites.push(sprite);
    }

    pub fn draw_sprites(&mut self) -> Result<(), GPUError> {
        let ctx = unsafe { &mut *self.ctx };
        let (img, acquire_sem, _idx, _) = ctx.acquire_new_image(&mut self.display)?;
        let targets = self.targets.clone();
        let sprites = self.sprites.clone();
        let pipeline = self.pso.pipeline;
        let bind_group0 = self.bind_groups[0].as_ref().map(|b| b.bind_group);
        let width = self.width;
        let height = self.height;

        self.command_list.record(|list| {
            for target in &targets {
                list.begin_drawing(&DrawBegin {
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
                    attachments: &target
                        .colors
                        .iter()
                        .map(|a| a.attachment)
                        .collect::<Vec<_>>(),
                }).unwrap();

                for sprite in &sprites {
                    let vb = sprite.vertex_buffer.expect("vb");
                    let draw = if let Some(ib) = sprite.index_buffer {
                        Command::DrawIndexed(DrawIndexed {
                            index_count: sprite.index_count as u32,
                            instance_count: 1,
                            vertices: vb,
                            indices: ib,
                            bind_groups: [
                                bind_group0,
                                None,
                                None,
                                None,
                            ],
                            ..Default::default()
                        })
                    } else {
                        Command::Draw(Draw {
                            count: sprite.index_count as u32,
                            instance_count: 1,
                            vertices: vb,
                            bind_groups: [
                                bind_group0,
                                None,
                                None,
                                None,
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

        ctx.present_display(&self.display, &self.semaphores)?;
        Ok(())
    }
}
