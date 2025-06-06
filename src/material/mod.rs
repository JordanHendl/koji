use dashi::{utils::Handle, *};
use std::collections::HashMap;
pub mod shader_reflection;
pub mod pipeline_builder;
pub mod bindless;

pub use pipeline_builder::*;
pub use shader_reflection::*;
pub use bindless::*;
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
            entries: &[VertexEntryInfo {
                format: ShaderPrimitiveType::Vec3,
                location: 0,
                offset: 0,
            }],
            stride: 12,
            rate: VertexRate::Vertex,
        };

        // Reflect descriptor bindings here (to be filled in next step)
        let bind_map = HashMap::new();
        let shader_infos: Vec<PipelineShaderInfo> =
            owned_shaders.iter().map(|s| s.as_info()).collect();
        let layout = ctx.make_graphics_pipeline_layout(&GraphicsPipelineLayoutInfo {
            debug_name: name,
            vertex_info: vertex_info.clone(),
            shaders: &shader_infos,
            bg_layouts: [None, None, None, None],
            details: GraphicsPipelineDetails {
                depth_test: Some(DepthInfo {
                    should_test: true,
                    should_write: true,
                }),
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


