extern crate spirv_cross;
use dashi::utils::Handle;
use dashi::{BindGroupLayout, Buffer, Image};
use spirv_cross::{hlsl, spirv};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BindingDetails {
    pub binding: u32,
    pub set: u32,
    pub descriptor_type: String,
    pub name: String,
}

type BindGroupLayoutCallback = Arc<dyn Fn(&BindingDetails) -> bool + Send + Sync>;

#[allow(dead_code)]
#[derive(Default)]
pub struct ShaderInspector {
    compiler_modules: Vec<spirv::Ast<spirv_cross::hlsl::Target>>,
}

#[allow(dead_code)]
impl ShaderInspector {
    pub fn new(spirvs: &[&[u32]]) -> Self {
        let mut s = Self {
            compiler_modules: Default::default(),
        };

        s.parse(spirvs).unwrap();
        s
    }
    /// Creates a new `ShaderInspector` from multiple SPIR-V binary slices.
    fn parse(&mut self, spirv_data_slices: &[&[u32]]) -> Result<(), &'static str> {
        self.compiler_modules.clear();
        let mut compiler_modules = Vec::new();
        for spirv_data in spirv_data_slices {
            let module = spirv::Module::from_words(spirv_data);
            let compiler = spirv::Ast::<hlsl::Target>::parse(&module)
                .map_err(|_| "Failed to create SPIR-V compiler")?;
            compiler_modules.push(compiler);
        }

        self.compiler_modules = compiler_modules;

        Ok(())
    }

    pub fn iter_binding_details<F>(&mut self, mut func: F)
    where
        F: FnMut(BindingDetails),
    {
        for compiler in &mut self.compiler_modules {
            if let Ok(resources) = compiler.get_shader_resources() {
                for resource in resources
                    .uniform_buffers
                    .iter()
                    .chain(&resources.storage_buffers)
                    .chain(&resources.sampled_images)
                    .chain(&resources.storage_images)
                {
                    let name = compiler.get_name(resource.id).unwrap_or_default();
                    let binding_info = compiler
                        .get_decoration(resource.id, spirv::Decoration::Binding)
                        .ok()
                        .unwrap_or_default();
                    let set_info = compiler
                        .get_decoration(resource.id, spirv::Decoration::DescriptorSet)
                        .ok()
                        .unwrap_or_default();
                    let descriptor_type = format!("{:?}", resource.type_id);

                    func(BindingDetails {
                        binding: binding_info,
                        set: set_info,
                        descriptor_type,
                        name,
                    });
                }
            }
        }
    }

    /// Combines all bindings across multiple SPIR-V modules and checks for a specific binding.
    pub fn get_binding_details(&mut self, binding_name: &str) -> Option<BindingDetails> {
        for compiler in &mut self.compiler_modules {
            if let Ok(resources) = compiler.get_shader_resources() {
                for resource in resources
                    .uniform_buffers
                    .iter()
                    .chain(&resources.storage_buffers)
                    .chain(&resources.sampled_images)
                    .chain(&resources.storage_images)
                {
                    let name = compiler.get_name(resource.id).unwrap_or_default();
                    if name == binding_name {
                        let binding_info = compiler
                            .get_decoration(resource.id, spirv::Decoration::Binding)
                            .ok()?;
                        let set_info = compiler
                            .get_decoration(resource.id, spirv::Decoration::DescriptorSet)
                            .ok()?;
                        let descriptor_type = format!("{:?}", resource.type_id);

                        return Some(BindingDetails {
                            binding: binding_info,
                            set: set_info,
                            descriptor_type,
                            name,
                        });
                    }
                }
            }
        }
        None
    }
}
