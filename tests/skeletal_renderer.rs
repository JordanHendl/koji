use koji::renderer::*;
use koji::gltf::{load_scene, MeshData};
use koji::material::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use koji::render_pass::RenderPassBuilder;
use glam::Mat4;
use koji::animation::Animator;
use inline_spirv::include_spirv;
use dashi::*;

#[cfg(feature = "gpu_tests")]
pub fn run_simple_skeleton() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();

    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let builder = RenderPassBuilder::new()
        .debug_name("MainPass")
        .color_attachment("color", Format::RGBA8)
        .subpass("main", ["color"], &[] as &[&str]);

    let mut renderer = Renderer::with_render_pass(320, 240, &mut ctx, builder).unwrap();
    renderer.add_canvas(canvas);

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh {
        MeshData::Skeletal(m) => m.clone(),
        _ => panic!("expected skeletal mesh"),
    };
    let bone_count = mesh.skeleton.bone_count();
    let instance = SkeletalInstance::new(&mut ctx, Animator::new(mesh.skeleton.clone())).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let vert: &[u32] = include_spirv!("src/renderer/skinning.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/renderer/skinning.frag", frag, glsl);
    let mut pso = PipelineBuilder::new(&mut ctx, "skinning_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(graph.output("color"))
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    let mats = vec![Mat4::IDENTITY; bone_count];
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.present_frame().unwrap();
    ctx.destroy();
}

#[cfg(feature = "gpu_tests")]
pub fn run_update_bones_twice() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let builder = RenderPassBuilder::new()
        .debug_name("MainPass")
        .color_attachment("color", Format::RGBA8)
        .subpass("main", ["color"], &[] as &[&str]);

    let mut renderer = Renderer::with_render_pass(320, 240, &mut ctx, builder).unwrap();
    renderer.add_canvas(canvas);

    let scene = load_scene("assets/data/simple_skin.gltf").expect("load");
    let mesh = match &scene.meshes[0].mesh {
        MeshData::Skeletal(m) => m.clone(),
        _ => panic!("expected skeletal mesh"),
    };
    let bone_count = mesh.skeleton.bone_count();
    let instance = SkeletalInstance::new(&mut ctx, Animator::new(mesh.skeleton.clone())).unwrap();
    renderer.register_skeletal_mesh(mesh, vec![instance], "skin".into());

    let vert: &[u32] = include_spirv!("src/renderer/skinning.vert", vert, glsl);
    let frag: &[u32] = include_spirv!("src/renderer/skinning.frag", frag, glsl);
    let mut pso = PipelineBuilder::new(&mut ctx, "skinning_pipeline")
        .vertex_shader(vert)
        .fragment_shader(frag)
        .render_pass(graph.output("color"))
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_skeletal_pso(pso, bgr);

    let mats = vec![Mat4::IDENTITY; bone_count];
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.update_skeletal_bones(0, 0, &mats);
    renderer.present_frame().unwrap();
    ctx.destroy();
}

#[cfg(all(test, feature = "gpu_tests"))]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    #[ignore]
    fn render_simple_skeleton() {
        run_simple_skeleton();
    }

    #[test]
    #[serial]
    #[ignore]
    fn update_bones_twice() {
        run_update_bones_twice();
    }
}
