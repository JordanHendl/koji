use koji::renderer::*;
use koji::material::ComputePipelineBuilder;
use dashi::*;
use inline_spirv::inline_spirv;

fn shader() -> Vec<u32> {
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
pub fn run() {
    let device = DeviceSelector::new()
        .unwrap()
        .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
        .unwrap_or_default();
    let mut ctx = Context::new(&ContextInfo { device }).unwrap();
    let mut renderer = Renderer::new(64, 64, "compute_test", &mut ctx).unwrap();

    let initial: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
    let buffer = ctx
        .make_buffer(&BufferInfo {
            debug_name: "buf",
            byte_size: (initial.len() * std::mem::size_of::<f32>()) as u32,
            visibility: MemoryVisibility::CpuAndGpu,
            usage: BufferUsage::STORAGE,
            initial_data: Some(bytemuck::cast_slice(&initial)),
        })
        .unwrap();
    renderer.resources().register_storage("data", buffer);
    renderer.resources().register_variable("addend", &mut ctx, 2.0f32);

    let spirv = shader();
    let mut pso = ComputePipelineBuilder::new(&mut ctx, "test_compute")
        .shader(&spirv)
        .build_with_resources(renderer.resources())
        .unwrap();
    let bgr = pso.create_bind_groups(renderer.resources()).unwrap();
    renderer.register_compute_pipeline("test", pso, bgr);

    renderer.queue_compute("test", [initial.len() as u32, 1, 1]);
    renderer.present_frame().unwrap();

    let slice = ctx.map_buffer::<u8>(buffer).unwrap();
    let results: &[f32] = bytemuck::cast_slice(&slice[..initial.len() * 4]);
    assert_eq!(results, &[3.0, 4.0, 5.0, 6.0]);
    ctx.unmap_buffer(buffer).unwrap();
    ctx.destroy();
}

#[cfg(all(test, feature = "gpu_tests"))]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    #[ignore]
    fn compute_pipeline() {
        run();
    }
}
