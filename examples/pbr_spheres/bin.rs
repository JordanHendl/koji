use dashi::utils::*;
use dashi::*;
use inline_spirv::include_spirv;
use koji::material::*;
use koji::renderer::*;
use koji::canvas::CanvasBuilder;
use koji::texture_manager;
use koji::utils::ResourceManager;
use koji::ResourceBinding;
use glam::*;
use winit::event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};
#[cfg(feature = "gpu_tests")]
use std::path::Path;

fn build_pbr_pipeline(
    ctx: &mut Context,
    target: koji::render_graph::GraphOutput,
) -> PSO {
    let vert: &[u32] = include_spirv!("assets/shaders/pbr_spheres.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("assets/shaders/pbr_spheres.frag", frag, glsl);
    PipelineBuilder::new(ctx, "pbr")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(target)
        .depth_enable(true)
        .cull_mode(CullMode::Back)
        .build()
}

fn make_sphere(lat: u32, long: u32) -> (Vec<Vertex>, Vec<u32>) {
    let vert_count = (lat + 1) * (long + 1);
    let idx_count = lat * long * 6;
    let mut verts = Vec::with_capacity(vert_count as usize);
    let mut idx = Vec::with_capacity(idx_count as usize);
    for i in 0..=lat {
        let v = i as f32 / lat as f32;
        let theta = v * std::f32::consts::PI;
        for j in 0..=long {
            let u = j as f32 / long as f32;
            let phi = u * std::f32::consts::TAU;
            let x = phi.cos() * theta.sin();
            let y = theta.cos();
            let z = phi.sin() * theta.sin();
            verts.push(Vertex {
                position: [x, y, z],
                normal: [x, y, z],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [u, v],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }
    for i in 0..lat {
        for j in 0..long {
            let a = i * (long + 1) + j;
            let b = a + long + 1;
            idx.extend_from_slice(&[a as u32, b as u32, a as u32 + 1]);
            idx.extend_from_slice(&[a as u32 + 1, b as u32, b as u32 + 1]);
        }
    }
    (verts, idx)
}

fn register_textures(ctx: &mut Context, res: &mut ResourceManager) {
    #[cfg(feature = "gpu_tests")]
    {
        let sampler = ctx.make_sampler(&SamplerInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            ..Default::default()
        }).unwrap();
        let albedo = texture_manager::load_from_file(
            ctx,
            res,
            "albedo_map",
            Default::default(),
            Path::new("assets/textures/albedo.png"),
        );
        let normal = texture_manager::load_from_file(
            ctx,
            res,
            "normal_map",
            Format::RGBA8Unorm,
            Path::new("assets/textures/normal.png"),
        );
        let metallic = texture_manager::load_from_file(
            ctx,
            res,
            "metallic_map",
            Format::RGBA8Unorm,
            Path::new("assets/textures/metallic.png"),
        );
        let roughness = texture_manager::load_from_file(
            ctx,
            res,
            "roughness_map",
            Format::RGBA8Unorm,
            Path::new("assets/textures/roughness.png"),
        );

        // Override texture bindings with combined samplers
        let a = *res.textures.get_ref(albedo);
        res.remove("albedo_map");
        res.register_combined("albedo_map", a.handle, a.view, a.dim, sampler);

        let n = *res.textures.get_ref(normal);
        res.remove("normal_map");
        res.register_combined("normal_map", n.handle, n.view, n.dim, sampler);

        let m = *res.textures.get_ref(metallic);
        res.remove("metallic_map");
        res.register_combined("metallic_map", m.handle, m.view, m.dim, sampler);

        let r = *res.textures.get_ref(roughness);
        res.remove("roughness_map");
        res.register_combined("roughness_map", r.handle, r.view, r.dim, sampler);
    }

    #[cfg(not(feature = "gpu_tests"))]
    {
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
        let defaults = [
            ("albedo_map", [255, 255, 255, 255]),
            ("normal_map", [127, 127, 255, 255]),
            ("metallic_map", [0, 0, 0, 255]),
            ("roughness_map", [255, 255, 255, 255]),
        ];
        for (key, color) in defaults {
            let handle = texture_manager::create_solid_color(ctx, res, key, color);
            let tex = *res.textures.get_ref(handle);
            res.remove(key);
            res.register_combined(key, tex.handle, tex.view, tex.dim, sampler);
        }
    }
}

pub fn run(ctx: &mut Context) {
    let canvas = CanvasBuilder::new()
        .extent([1920, 1080])
        .color_attachment("color", Format::RGBA8)
        .depth_attachment("depth", Format::D24S8)
        .build(ctx)
        .unwrap();

    let mut renderer = Renderer::with_canvas(1920, 1080, ctx, canvas).unwrap();
    renderer.set_clear_depth(1.0);
    register_textures(ctx, renderer.resources());

    let mut pso = build_pbr_pipeline(ctx, renderer.graph().output("color"));

    let proj =
        Mat4::perspective_rh_gl(45.0_f32.to_radians(), 1920.0 / 1080.0, 0.1, 100.0);
    let cam_pos = Vec3::new(0.0, 0.0, 5.0);
    let view = Mat4::look_at_rh(cam_pos, Vec3::ZERO, Vec3::Y);
    renderer.set_camera(0, proj * view, cam_pos);

    let mut light_pos = Vec3::new(0.0, 0.0, 5.0);
    let mut light = LightDesc {
        position: light_pos.into(),
        intensity: 1.0,
        ..Default::default()
    };
    renderer
        .resources()
        .register_variable("SceneLight", ctx, light);

    // Create bind groups now that all resources are registered
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let (base_verts, inds) = make_sphere(32, 32);
    let offsets = [-3.0f32, 0.0, 3.0];
    for offset in offsets {
        let mut verts = base_verts.clone();
        for v in &mut verts {
            v.position[0] += offset;
        }
        let mesh = StaticMesh {
            material_id: "pbr".into(),
            vertices: verts,
            indices: Some(inds.clone()),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        };
        renderer.register_static_mesh(mesh, None, "pbr".into());
    }

    let mut angle: f32 = 0.0;

    renderer.render_loop(|r, event| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match key {
                    VirtualKeyCode::Left => light_pos.x -= 0.2,
                    VirtualKeyCode::Right => light_pos.x += 0.2,
                    VirtualKeyCode::Up => light_pos.y += 0.2,
                    VirtualKeyCode::Down => light_pos.y -= 0.2,
                    _ => {}
                },
                _ => {}
            },
            Event::MainEventsCleared => {
                angle += r.time_stats().delta_time * 0.25;
                let radius = 10.0;
                let eye = Vec3::new(angle.cos() * radius, -2.0, angle.sin() * radius);
                let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
                let view_proj = proj * view;
                r.set_camera(0, view_proj, eye);

                if let Some(ResourceBinding::Uniform(buf)) = r.resources().get("SceneLight") {
                    light.position = light_pos.into();
                    let slice = ctx.map_buffer_mut(*buf).unwrap();
                    let bytes = bytemuck::bytes_of(&light);
                    slice[..bytes.len()].copy_from_slice(bytes);
                    ctx.unmap_buffer(*buf).unwrap();
                }
            }
            _ => {}
        }
    });
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
