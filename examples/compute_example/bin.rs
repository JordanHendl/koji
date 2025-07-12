use dashi::*;
use inline_spirv::inline_spirv;
use koji::renderer::*;
use koji::material::ComputePipelineBuilder;

fn compute_spirv() -> Vec<u32> {
    inline_spirv!(
        r"#version 450
        layout(local_size_x = 1) in;
        layout(set = 0, binding = 0) buffer Data { float values[]; } data;
        layout(set = 0, binding = 1) uniform Add { float val; } addend;
        void main() {
            uint idx = gl_GlobalInvocationID.x;
            data.values[idx] += addend.val;
        }",
        comp
    ).to_vec()
}

#[cfg(feature = "gpu_tests")]
pub fn run(ctx: &mut Context) {
    let mut renderer = Renderer::new(64, 64, "compute", ctx).unwrap();

    let input: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
    let buffer = ctx
        .make_buffer(&BufferInfo {
            debug_name: "compute_buffer",
            byte_size: (input.len() * std::mem::size_of::<f32>()) as u32,
            visibility: MemoryVisibility::CpuAndGpu,
            usage: BufferUsage::STORAGE,
            initial_data: Some(bytemuck::cast_slice(&input)),
        })
        .unwrap();
    renderer.resources().register_storage("data", buffer);
    renderer.resources().register_variable("addend", ctx, 2.0f32);

    let shader = compute_spirv();
    let mut pso = ComputePipelineBuilder::new(ctx, "compute_add")
        .shader(&shader)
        .build_with_resources(renderer.resources())
        .unwrap();
    let bgr = pso.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_compute_pipeline("add", pso, bgr);

    renderer.queue_compute("add", [input.len() as u32, 1, 1]);
    renderer.present_frame().unwrap();

    let slice = ctx.map_buffer::<u8>(buffer).unwrap();
    let results: &[f32] = bytemuck::cast_slice(&slice[..input.len() * 4]);
    println!("results: {:?}", results);
    ctx.unmap_buffer(buffer).unwrap();
}

fn main() {
    #[cfg(feature = "gpu_tests")]
    {
        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = Context::new(&ContextInfo { device }).unwrap();
        run(&mut ctx);
        ctx.destroy();
    }
}
