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
    let mut renderer = Renderer::new(640, 480, "pbr_spheres", ctx).unwrap();

    let normal: [u8; 4] = [128, 128, 255, 255];
    let (base_verts, inds) = make_sphere(16, 32);
    let positions = [-2.0f32, 0.0, 2.0];
    let colors = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]];
    let metallics = [0u8, 128u8, 255u8];
    let roughness = [64u8, 128u8, 200u8];

    for i in 0..3 {
        let alb_img = ctx
            .make_image(&ImageInfo {
                debug_name: "alb",
                dim: [1, 1, 1],
                format: Format::RGBA8,
                mip_levels: 1,
                layers: 1,
                initial_data: Some(&colors[i]),
            })
            .unwrap();
        let alb_view = ctx
            .make_image_view(&ImageViewInfo {
                img: alb_img,
                ..Default::default()
            })
            .unwrap();

        let norm_img = ctx
            .make_image(&ImageInfo {
                debug_name: "norm",
                dim: [1, 1, 1],
                format: Format::RGBA8,
                mip_levels: 1,
                layers: 1,
                initial_data: Some(&normal),
            })
            .unwrap();
        let norm_view = ctx
            .make_image_view(&ImageViewInfo {
                img: norm_img,
                ..Default::default()
            })
            .unwrap();

        let metal_img = ctx
            .make_image(&ImageInfo {
                debug_name: "metal",
                dim: [1, 1, 1],
                format: Format::RGBA8,
                mip_levels: 1,
                layers: 1,
                initial_data: Some(&[metallics[i], metallics[i], metallics[i], 255]),
            })
            .unwrap();
        let metal_view = ctx
            .make_image_view(&ImageViewInfo {
                img: metal_img,
                ..Default::default()
            })
            .unwrap();

        let rough_img = ctx
            .make_image(&ImageInfo {
                debug_name: "rough",
                dim: [1, 1, 1],
                format: Format::RGBA8,
                mip_levels: 1,
                layers: 1,
                initial_data: Some(&[roughness[i], roughness[i], roughness[i], 255]),
            })
            .unwrap();
        let rough_view = ctx
            .make_image_view(&ImageViewInfo {
                img: rough_img,
                ..Default::default()
            })
            .unwrap();

        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();

        renderer
            .resources()
            .register_combined("albedo_map", alb_img, alb_view, [1, 1], sampler);
        renderer
            .resources()
            .register_combined("normal_map", norm_img, norm_view, [1, 1], sampler);
        renderer
            .resources()
            .register_combined("metallic_map", metal_img, metal_view, [1, 1], sampler);
        renderer
            .resources()
            .register_combined("roughness_map", rough_img, rough_view, [1, 1], sampler);

        let mut pso = build_pbr_pipeline(ctx, renderer.render_pass(), 0);
        let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
        let mat_id = format!("pbr{}", i);
        renderer.register_material_pipeline(&mat_id, pso, bgr);

        let mut verts = base_verts.clone();
        for v in &mut verts {
            v.position[0] += positions[i];
        }
        let mesh = StaticMesh {
            material_id: mat_id.clone(),
            vertices: verts,
            indices: Some(inds.clone()),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        };
        renderer.register_static_mesh(mesh, None, mat_id);
    }

    renderer.render_loop(|_r| {});
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
