use crate::material::*;
use dashi::*;
use spirv_reflect::types::{ReflectDescriptorType, ReflectTypeDescription};
use spirv_reflect::ShaderModule;
use std::{collections::HashMap, fs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderDescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    InputAttachment,
    AccelerationStructure,
    Unknown,
}

#[derive(Debug)]
pub struct ShaderSpecializationConstant {
    pub id: u32,
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug)]
pub struct ShaderReflectionInfo {
    pub bindings: HashMap<u32, Vec<ShaderDescriptorBinding>>,
    pub push_constants: Vec<ShaderPushConstant>,
}

#[derive(Debug)]
pub struct ShaderDescriptorBinding {
    pub name: String,
    pub binding: u32,
    pub set: u32,
    pub ty: ShaderDescriptorType,
    /// total size of the block in bytes
    pub block_size: u32,
    /// for a struct: a list of (field_name, byte_offset, field_size)
    pub members: Vec<(String, u32, u32)>,
}

#[derive(Debug)]
pub struct ShaderPushConstant {
    pub offset: u32,
    pub size: u32,
}

pub fn reflect_shader(spirv: &[u32]) -> ShaderReflectionInfo {
    if spirv.is_empty() {
        return ShaderReflectionInfo {
            bindings: Default::default(),
            push_constants: Default::default(),
        };
    }

    let module = ShaderModule::load_u32_data(spirv).expect("Failed to parse SPIR-V");

    let mut bindings: HashMap<u32, Vec<ShaderDescriptorBinding>> = HashMap::new();
    if let Ok(descs) = module.enumerate_descriptor_bindings(None) {
        for desc in descs {
            // pull out block info if this is a UBO/SSBO
            let (block_size, members) = if !desc.block.members.is_empty() {
                let size = desc.block.size;
                let mems = desc
                    .block
                    .members
                    .into_iter()
                    .map(|m| (m.name, m.offset, m.size))
                    .collect();
                (size, mems)
            } else {
                // not a block (e.g. a sampler), size = 0, no members
                (0, Vec::new())
            };

            let entry = bindings.entry(desc.set).or_default();
            entry.push(ShaderDescriptorBinding {
                name: desc.name.clone(),
                binding: desc.binding,
                set: desc.set,
                ty: map_descriptor_type(desc.descriptor_type),
                block_size,
                members,
            });
        }
    }

    let mut push_constants = Vec::new();
    if let Ok(ranges) = module.enumerate_push_constant_blocks(None) {
        for block in ranges {
            push_constants.push(ShaderPushConstant {
                offset: block.offset,
                size: block.size,
            });
        }
    }

    ShaderReflectionInfo {
        bindings,
        push_constants,
    }
}

fn map_descriptor_type(ty: ReflectDescriptorType) -> ShaderDescriptorType {
    use spirv_reflect::types::ReflectDescriptorType;
    use ShaderDescriptorType::*;
    match ty {
        ReflectDescriptorType::Sampler => Sampler,
        ReflectDescriptorType::CombinedImageSampler => CombinedImageSampler,
        ReflectDescriptorType::SampledImage => SampledImage,
        ReflectDescriptorType::StorageImage => StorageImage,
        ReflectDescriptorType::UniformTexelBuffer => UniformTexelBuffer,
        ReflectDescriptorType::StorageTexelBuffer => StorageTexelBuffer,
        ReflectDescriptorType::UniformBuffer => UniformBuffer,
        ReflectDescriptorType::StorageBuffer => StorageBuffer,
        ReflectDescriptorType::UniformBufferDynamic => UniformBufferDynamic,
        ReflectDescriptorType::StorageBufferDynamic => StorageBufferDynamic,
        ReflectDescriptorType::InputAttachment => InputAttachment,
        _ => Unknown,
    }
}

fn descriptor_type_to_string(ty: ReflectDescriptorType) -> String {
    use ReflectDescriptorType::*;
    match ty {
        Sampler => "sampler".to_string(),
        CombinedImageSampler => "combined_image_sampler".to_string(),
        SampledImage => "sampled_image".to_string(),
        StorageImage => "storage_image".to_string(),
        UniformTexelBuffer => "uniform_texel_buffer".to_string(),
        StorageTexelBuffer => "storage_texel_buffer".to_string(),
        UniformBuffer => "uniform_buffer".to_string(),
        StorageBuffer => "storage_buffer".to_string(),
        UniformBufferDynamic => "uniform_buffer_dynamic".to_string(),
        StorageBufferDynamic => "storage_buffer_dynamic".to_string(),
        InputAttachment => "input_attachment".to_string(),
        AccelerationStructureKHR => "acceleration_structure_khr".to_string(),
        _ => format!("unknown({:?})", ty),
    }
}
