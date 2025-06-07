use koji::material::*;
use koji::renderer::*;
use koji::animation::*;
use dashi::*;
use glam::Mat4;
use serial_test::serial;

fn make_vertex(pos:[f32;3], col:[f32;4]) -> SkeletalVertex {
    SkeletalVertex {
        position: pos,
        normal: [0.0,0.0,1.0],
        tangent: [1.0,0.0,0.0,1.0],
        uv: [0.0,0.0],
        color: col,
        joint_indices: [0,0,0,0],
        joint_weights: [1.0,0.0,0.0,0.0],
    }
}

fn simple_mesh() -> SkeletalMesh {
    let verts = vec![
        make_vertex([0.0,-0.5,0.0],[1.0,0.0,0.0,1.0]),
        make_vertex([0.5,0.5,0.0],[0.0,1.0,0.0,1.0]),
        make_vertex([-0.5,0.5,0.0],[0.0,0.0,1.0,1.0]),
    ];
    SkeletalMesh {
        vertices: verts,
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
        skeleton: Skeleton { bones: vec![Bone{ name:"root".into(), parent: None, inverse_bind: Mat4::IDENTITY}] },
        bone_buffer: None,
    }
}

#[test]
#[serial]
#[ignore]
fn render_skeletal_triangle(){
    let device = DeviceSelector::new().unwrap().select(DeviceFilter::default().add_required_type(DeviceType::Dedicated)).unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();
    let mut renderer = Renderer::new(320,240,"skel", &mut ctx).expect("renderer");

    let mut mesh = simple_mesh();
    mesh.upload(&mut ctx).unwrap();
    let bone_buf = mesh.bone_buffer.unwrap();
    renderer.register_skeletal_mesh(mesh);

    renderer.resources().register_storage("bone_buf", bone_buf);
    let mut pso = build_skinning_pipeline(&mut ctx, renderer.render_pass(), 0);
    let bgr = pso.create_bind_groups(&renderer.resources());
    renderer.register_pso(RenderStage::Skinned, pso, bgr);

    let slice = unsafe { ctx.map_buffer_mut(bone_buf).unwrap() };
    let mat = Mat4::IDENTITY.to_cols_array();
    let bytes = bytemuck::bytes_of(&mat);
    slice[..bytes.len()].copy_from_slice(bytes);
    ctx.unmap_buffer(bone_buf).unwrap();

    renderer.present_frame().unwrap();
    ctx.destroy();
}

