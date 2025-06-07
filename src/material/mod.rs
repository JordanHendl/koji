use dashi::{utils::Handle, *};
use std::collections::HashMap;
pub mod shader_reflection;
pub mod pipeline_builder;
pub mod bindless;
pub mod bindless_lighting;

#[cfg(test)]
mod pipeline_builder_tests;

pub use pipeline_builder::*;
pub use shader_reflection::*;
pub use bindless::*;
pub use bindless_lighting::*;
use crate::utils::ResourceManager;

pub struct MaterialPipeline {
    pub name: String,
    pub pipeline: Handle<GraphicsPipeline>,
    pub layout: Handle<GraphicsPipelineLayout>,
    pub bind_map: HashMap<String, u32>, // Maps material names to binding slots
    pub vertex_info: VertexDescriptionInfo<'static>,
}

impl MaterialPipeline {
    pub fn from_yaml(
        ctx: &mut Context,
        res: &mut ResourceManager,
        yaml: &serde_yaml::Mapping,
        render_pass: Handle<RenderPass>,
        subpass_id: u32,
    ) -> Result<Self, GPUError> {
        use ShaderType::*;
        pub struct OwnedPipelineShaderInfo {
            pub stage: ShaderType,
            pub spirv: Vec<u32>,
        }
        impl OwnedPipelineShaderInfo {
            pub fn as_info(&self) -> PipelineShaderInfo {
                PipelineShaderInfo {
                    stage: self.stage,
                    spirv: &self.spirv,
                    specialization: &[],
                }
            }
        }

        let name = yaml
            .get("name")
            .expect("Missing name section")
            .as_str()
            .unwrap_or("material");
        let shaders = yaml.get("shaders").expect("Missing 'shaders' section");
        let shaders_map = shaders.as_mapping().expect("Expected mapping");

        let mut owned_shaders = Vec::new();

        for (stage_str, path_val) in shaders_map {
            let stage = match stage_str.as_str().unwrap() {
                "vertex" => ShaderType::Vertex,
                "fragment" => ShaderType::Fragment,
                //   "geometry" => ShaderType::Geometry,
                //   "tess_control" => ShaderType::TessellationControl,
                //   "tess_eval" => ShaderType::TessellationEvaluation,
                other => panic!("Unknown shader stage '{}'", other),
            };

            let path = path_val.as_str().unwrap();
            let spirv = std::fs::read(path).expect("Failed to read SPIR-V file");
            owned_shaders.push(OwnedPipelineShaderInfo {
                stage,
                spirv: unsafe { spirv.align_to::<u32>() }.1.to_vec(),
            });
        }

        let vertex_info = VertexDescriptionInfo {
            entries: &[
                VertexEntryInfo { format: ShaderPrimitiveType::Vec3, location: 0, offset: 0 },
                VertexEntryInfo { format: ShaderPrimitiveType::Vec3, location: 1, offset: 12 },
                VertexEntryInfo { format: ShaderPrimitiveType::Vec4, location: 2, offset: 24 },
                VertexEntryInfo { format: ShaderPrimitiveType::Vec2, location: 3, offset: 40 },
                VertexEntryInfo { format: ShaderPrimitiveType::Vec4, location: 4, offset: 48 },
            ],
            stride: 64,
            rate: VertexRate::Vertex,
        };

        // Reflect descriptor bindings and create layouts
        let vert_spv = owned_shaders
            .iter()
            .find(|s| matches!(s.stage, ShaderType::Vertex))
            .map(|s| s.spirv.as_slice())
            .unwrap_or(&[]);
        let frag_spv = owned_shaders
            .iter()
            .find(|s| matches!(s.stage, ShaderType::Fragment))
            .map(|s| s.spirv.as_slice())
            .unwrap_or(&[]);

        let vert_info = reflect_shader(vert_spv);
        let frag_info = reflect_shader(frag_spv);
        let mut combined: HashMap<u32, Vec<ShaderDescriptorBinding>> = HashMap::new();
        for (set, binds) in vert_info.bindings.into_iter().chain(frag_info.bindings) {
            combined.entry(set).or_default().extend(binds);
        }
        let mut bind_map = HashMap::new();
        let mut bg_layouts: [Option<Handle<BindGroupLayout>>; 4] = [None, None, None, None];
        for (set, binds) in combined.iter() {
            let mut vars = Vec::new();
            for b in binds {
                bind_map.insert(b.name.clone(), b.binding);
                vars.push(BindGroupVariable {
                    var_type: descriptor_to_var_type(b.ty),
                    binding: b.binding,
                    count: b.count,
                });
            }
            let info = BindGroupLayoutInfo {
                debug_name: name,
                shaders: &[ShaderInfo {
                    shader_type: ShaderType::All,
                    variables: &vars,
                }],
            };
            bg_layouts[*set as usize] = Some(ctx.make_bind_group_layout(&info)?);
        }

        let shader_infos: Vec<PipelineShaderInfo> = owned_shaders.iter().map(|s| s.as_info()).collect();
        let layout = ctx.make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
            debug_name: name,
            vertex_info: vertex_info.clone(),
            shaders: &shader_infos,
            bg_layouts,
            details: GraphicsPipelineDetails {
                depth_test: Some(DepthInfo { should_test: true, should_write: true }),
                ..Default::default()
            },
        })?;

        let pipeline = ctx.make_graphics_pipeline(&GraphicsPipelineInfo {
            layout,
            render_pass,
            subpass_id: subpass_id as u8,
            debug_name: name,
            ..Default::default()
        })?;

        Ok(Self {
            name: name.to_string(),
            pipeline,
            layout,
            vertex_info,
            bind_map,
        })
    }
}


