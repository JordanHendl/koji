use dashi::utils::*;
use dashi::*;
use inline_spirv::include_spirv;
use koji::material::*;
use koji::renderer::*;
use koji::texture_manager;
use koji::utils::ResourceManager;
#[cfg(feature = "gpu_tests")]
use std::path::Path;

fn build_pbr_pipeline(ctx: &mut Context, rp: Handle<RenderPass>, subpass: u32) -> PSO {
    let vert: &[u32] = include_spirv!("assets/shaders/pbr.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("assets/shaders/pbr.frag", frag, glsl);
    PipelineBuilder::new(ctx, "pbr")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(rp, subpass)
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
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
        let albedo = texture_manager::load_from_file(
            ctx,
            res,
            "albedo_map",
            Path::new("assets/textures/albedo.png"),
        );
        let normal = texture_manager::load_from_file(
            ctx,
            res,
            "normal_map",
            Path::new("assets/textures/normal.png"),
        );
        let metallic = texture_manager::load_from_file(
            ctx,
            res,
            "metallic_map",
            Path::new("assets/textures/metallic.png"),
        );
        let roughness = texture_manager::load_from_file(
            ctx,
            res,
            "roughness_map",
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
    let mut renderer = Renderer::new(1920, 1080, "pbr_spheres", ctx).unwrap();
    register_textures(ctx, renderer.resources());

    let mut pso = build_pbr_pipeline(ctx, renderer.render_pass(), 0);
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

    renderer.render_loop(|_r| {
        // No per-frame updates required for this sample
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
