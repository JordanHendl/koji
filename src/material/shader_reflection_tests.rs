use super::*;
use dashi::gpu::structs::BindGroupVariableType;
use inline_spirv::inline_spirv;

#[test]
fn reflect_descriptor_bindings() {
    let spirv: Vec<u32> = inline_spirv!(
        r#"
        #version 450
        layout(set=0,binding=0) uniform sampler s;
        layout(set=0,binding=1) uniform texture2D tex[2];
        layout(set=1,binding=0) uniform sampler2D combo;
        layout(set=1,binding=1) buffer Buf { float data[]; } buf;
        layout(set=2,binding=5) uniform Block { vec4 v; } block;
        void main() {}
        "#,
        comp
    )
    .to_vec();

    let info = reflect_shader(&spirv);
    assert_eq!(info.bindings.len(), 3);

    let set0 = &info.bindings[&0];
    assert!(set0.iter().any(|b| b.name == "s" && b.binding == 0 && b.set == 0 && b.count == 1));
    assert!(set0.iter().any(|b| b.name == "tex" && b.binding == 1 && b.set == 0 && b.count == 2));

    let set1 = &info.bindings[&1];
    assert!(set1.iter().any(|b| b.name == "combo" && b.binding == 0 && b.set == 1 && b.count == 1));
    assert!(set1.iter().any(|b| b.name == "buf" && b.binding == 1 && b.set == 1 && b.count == 1));

    let set2 = &info.bindings[&2];
    assert!(set2.iter().any(|b| b.name == "block" && b.binding == 5 && b.set == 2 && b.count == 1));
}

#[test]
fn descriptor_var_type_mappings() {
    use ShaderDescriptorType::*;
    assert_eq!(
        descriptor_to_var_type(SampledImage),
        BindGroupVariableType::SampledImage
    );
    assert_eq!(
        descriptor_to_var_type(CombinedImageSampler),
        BindGroupVariableType::SampledImage
    );
    assert_eq!(
        descriptor_to_var_type(UniformBuffer),
        BindGroupVariableType::Uniform
    );
    assert_eq!(
        descriptor_to_var_type(StorageBuffer),
        BindGroupVariableType::Storage
    );
    assert_eq!(
        descriptor_to_var_type(StorageImage),
        BindGroupVariableType::StorageImage
    );
}

#[test]
fn shader_without_resources_has_empty_reflection() {
    let spirv: Vec<u32> = inline_spirv!(
        r"#version 450
        void main() {}",
        vert
    )
    .to_vec();

    let info = reflect_shader(&spirv);
    assert!(info.bindings.is_empty());
    assert!(info.push_constants.is_empty());
}

