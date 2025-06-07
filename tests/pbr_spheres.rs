use koji::material::*;
use koji::renderer::*;
use koji::utils::*;
use dashi::*;
use serial_test::serial;
use inline_spirv::include_spirv;
use koji::material::pipeline_builder::PipelineBuilder;
use dashi::utils::Handle;
use std::f32::consts::{PI, TAU};

fn build_pbr_pipeline(ctx: &mut Context, rp: Handle<RenderPass>, subpass: u32) -> PSO {
    let vert: &[u32] = include_spirv!("src/material/pbr.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/material/pbr.frag", frag, glsl);
    PipelineBuilder::new(ctx, "pbr_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(rp, subpass)
        .depth_enable(true)
        .cull_mode(CullMode::Back)
        .build()
}

fn sphere_mesh(res: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    for y in 0..=res {
        let v = y as f32 / res as f32;
        let phi = PI * v;
        for x in 0..=res {
            let u = x as f32 / res as f32;
            let theta = TAU * u;
            let (s, c) = phi.sin_cos();
            let (st, ct) = theta.sin_cos();
            let pos = [ct * s, c, st * s];
            verts.push(Vertex {
                position: pos,
                normal: pos,
                tangent: [1.0,0.0,0.0,1.0],
                uv: [u,v],
                color: [1.0,1.0,1.0,1.0],
            });
        }
    }
    let mut indices = Vec::new();
    let row = res + 1;
    for y in 0..res {
        for x in 0..res {
            let i0 = y * row + x;
            let i1 = i0 + 1;
            let i2 = i0 + row;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0,i2,i1,i1,i2,i3]);
        }
    }
    (verts, indices)
}

#[test]
#[serial]
#[ignore]
fn pbr_spheres() {
    let device = DeviceSelector::new().unwrap().select(DeviceFilter::default().add_required_type(DeviceType::Dedicated)).unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"pbr_spheres", &mut ctx).expect("renderer");

    let mut pso = build_pbr_pipeline(&mut ctx, renderer.render_pass(),0);
    let bgr = pso.create_bind_groups(&renderer.resources());
    renderer.register_pso(RenderStage::Opaque, pso, bgr);

    let colors = [[255,0,0,255],[0,255,0,255],[0,0,255,255]];
    for (i,color) in colors.iter().enumerate() {
        let img = ctx.make_image(&ImageInfo{debug_name:"alb", dim:[1,1,1], format:Format::RGBA8, mip_levels:1, layers:1, initial_data:Some(color)}).unwrap();
        let view = ctx.make_image_view(&ImageViewInfo{img, ..Default::default()}).unwrap();
        let sampler = ctx.make_sampler(&SamplerInfo::default()).unwrap();
        renderer.resources().register_combined(format!("albedo_map_{}",i), img, view,[1,1], sampler);
    }

    let (verts, inds) = sphere_mesh(8);
    for _ in 0..3 {
        let mesh = StaticMesh { vertices: verts.clone(), indices: Some(inds.clone()), vertex_buffer: None, index_buffer: None, index_count: 0 };
        renderer.register_static_mesh(mesh,None);
    }

    renderer.present_frame().unwrap();
    ctx.destroy();
}

