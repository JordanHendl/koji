use dashi::utils::*;
use dashi::*;
use inline_spirv::include_spirv;
use koji::material::*;
use koji::renderer::*;

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

pub fn run(ctx: &mut Context) {
    let mut renderer = Renderer::new(320, 240, "pbr_spheres", ctx).unwrap();
    let mut pso = build_pbr_pipeline(ctx, renderer.render_pass(), 0);
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pso(RenderStage::Opaque, pso, bgr);

    let (verts, inds) = make_sphere(8, 16);
    for _ in 0..3 {
        let mesh = StaticMesh {
            material_id: "pbr".into(),
            vertices: verts.clone(),
            indices: Some(inds.clone()),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        };
        renderer.register_static_mesh(mesh, None, "pbr".into());
    }

    let white: [u8; 4] = [255, 255, 255, 255];
    let img = ctx
        .make_image(&ImageInfo {
            debug_name: "alb",
            dim: [1, 1, 1],
            format: Format::RGBA8,
            mip_levels: 1,
            layers: 1,
            initial_data: Some(&white),
        })
        .unwrap();
    let view = ctx
        .make_image_view(&ImageViewInfo {
            img,
            ..Default::default()
        })
        .unwrap();
    let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
    renderer
        .resources()
        .register_combined("albedo_map", img, view, [1, 1], sampler);
    renderer
        .resources()
        .register_combined("normal_map", img, view, [1, 1], sampler);
    renderer
        .resources()
        .register_combined("metallic_map", img, view, [1, 1], sampler);
    renderer
        .resources()
        .register_combined("roughness_map", img, view, [1, 1], sampler);

    renderer.present_frame().unwrap();
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
