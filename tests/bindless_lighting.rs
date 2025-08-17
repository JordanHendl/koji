use koji::material::*;
use koji::renderer::*;
use koji::canvas::CanvasBuilder;
use koji::render_graph::RenderGraph;
use dashi::*;
use inline_spirv::inline_spirv;

fn vert() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(location=0) in vec3 pos;
        void main(){ gl_Position=vec4(pos,1.0); }",
        vert
    ).to_vec()
}

fn frag() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(set=0,binding=0) buffer Lights { vec4 data[]; };
        layout(location=0) out vec4 o;
        void main(){ o = data.length()>0 ? data[0] : vec4(1.0); }",
        frag
    ).to_vec()
}

#[cfg(feature = "gpu_tests")]
pub fn run() {
    let device = DeviceSelector::new().unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo{ device }).unwrap();

    let canvas = CanvasBuilder::new()
        .extent([320, 240])
        .color_attachment("color", Format::RGBA8)
        .build(&mut ctx)
        .unwrap();
    let mut graph = RenderGraph::new();
    graph.add_canvas(&canvas);

    let mut renderer = Renderer::with_graph(320, 240, &mut ctx, graph).unwrap();

    let mut pso = PipelineBuilder::new(&mut ctx, "lights")
        .vertex_shader(&vert())
        .fragment_shader(&frag())
        .render_pass(renderer.graph().output("color"))
        .build();
    let bgr = pso.create_bind_groups(&renderer.resources()).unwrap();
    renderer.register_pipeline_for_pass("main", pso, bgr);

    let mut lights = BindlessLights::new();
    let light = LightDesc{ position:[0.0,0.0,0.0], intensity:1.0, color:[1.0,1.0,1.0], range:1.0, direction:[0.0,0.0,-1.0], _pad:0 };
    lights.add_light(&mut ctx, renderer.resources(), light);
    lights.register(renderer.resources());

    let mesh = StaticMesh {
        material_id: "lighting".into(),
        vertices: vec![
            Vertex{position:[-0.5,-0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.0,0.0],color:[1.0,1.0,1.0,1.0]},
            Vertex{position:[0.5,-0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[1.0,0.0],color:[1.0,1.0,1.0,1.0]},
            Vertex{position:[0.0,0.5,0.0],normal:[0.0,0.0,1.0],tangent:[1.0,0.0,0.0,1.0],uv:[0.5,1.0],color:[1.0,1.0,1.0,1.0]},
        ],
        indices: None,
        vertex_buffer: None,
        index_buffer: None,
        index_count: 0,
    };
    let out = renderer.graph().output("color");
    renderer.register_static_mesh(mesh,None,"lighting".into(), out);

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
    fn bindless_lighting_sample() {
        run();
    }
}
