use dashi::{
    utils::Handle, BindGroup, BindGroupInfo, BindGroupLayout, BindGroupLayoutInfo,
    BindGroupVariable, BindGroupVariableType, BindingInfo, Context, ShaderInfo, ShaderResource,
    ShaderType,
};

mod timedate;
use reflection::ShaderInspector;

use crate::modules::timedate::TimeDate;

mod reflection;
pub type ShaderStageCallback = Box<dyn Fn() -> Vec<u32> + 'static>;

pub trait ShaderModule {
    fn resource(&self, name: &str) -> Option<ShaderResource>;
    fn update(&mut self);
}
pub struct GraphicsPipelineInfo {
    pub vertex: ShaderStageCallback,
    pub fragment: ShaderStageCallback,
}

pub struct PipelineModule {
    pub bind_group: Handle<BindGroup>,
}

pub struct PipelineModuleManager {
    ctx: *mut Context,
    modules: Vec<Box<dyn ShaderModule>>,
}

impl PipelineModuleManager {
    pub fn pipeline_module(&mut self, info: &GraphicsPipelineInfo) -> PipelineModule {
        let loaded_spirvs = vec![(info.vertex)(), (info.fragment)()];
        let spirvs: Vec<&[u32]> = loaded_spirvs.iter().map(|a| a.as_slice()).collect();
        let mut inspector = ShaderInspector::new(&spirvs);

        let mut bind_groups: [Option<Handle<BindGroup>>; 4] = Default::default();
        let mut layouts: [Option<Handle<BindGroupLayout>>; 4] = Default::default();
        let mut bindings: [Vec<BindingInfo>; 4] = Default::default();
        let mut layout_info: [Vec<BindGroupVariable>; 4] = Default::default();

        inspector.iter_binding_details(|b| {
            layout_info[b.set as usize].push(BindGroupVariable {
                var_type: match b.descriptor_type.as_str() {
                    "uniform" => BindGroupVariableType::Uniform,
                    "storage" => BindGroupVariableType::Storage,
                    "sampled_image" => BindGroupVariableType::SampledImage,
                    "storage_image" => BindGroupVariableType::StorageImage,
                    _ => panic!("Unknown Descriptor Type!"),
                },
                binding: b.binding,
            });

            for m in &self.modules {
                if let Some(res) = m.resource(&b.name) {
                    bindings[b.set as usize].push(BindingInfo {
                        resource: res,
                        binding: b.binding,
                    });
                }
            }
        });

        for (i, b) in bindings.iter().enumerate() {
            if !b.is_empty() {
                let layout = ctx
                    .make_bind_group_layout(&BindGroupLayoutInfo {
                        debug_name: "[KOJI] Shader Bind Group Layout",
                        shaders: &[ShaderInfo {
                            shader_type: ShaderType::All,
                            variables: &layout_info[i],
                        }],
                    })
                    .unwrap();

                let bg = ctx
                    .make_bind_group(&BindGroupInfo {
                        debug_name: "[KOJI] Shader Bind Group",
                        layout,
                        bindings: &bindings[i],
                        set: i as u32,
                    })
                    .unwrap();

                layouts[i] = Some(layout);
                bind_groups[i] = Some(bg);
            }
        }
        todo!()
    }

    pub fn new(ctx: &mut Context, info: &GraphicsPipelineInfo) -> Self {
        let mut s = Self {
            ctx,
            modules: vec![Box::new(TimeDate::new())],
        };

        s.make_shader_modules();

        s
    }

    fn make_shader_modules(&mut self) {}
    pub fn register_graphics_pipeline(&mut self, name: &str, info: &GraphicsPipelineInfo) {}
    pub fn generate_bindings(&mut self, name: &str) -> Option<PipelineBinding> {
        todo!()
    }
}
