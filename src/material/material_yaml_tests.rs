use super::*;
use crate::utils::ResourceManager;
use dashi::builders::RenderPassBuilder;
use inline_spirv::inline_spirv;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;

fn make_ctx() -> Context {
    Context::headless(&ContextInfo::default()).unwrap()
}

fn simple_vert() -> Vec<u32> {
    inline_spirv!(
        r#"
        #version 450
        layout(location=0) in vec2 pos;
        layout(set=0, binding=0) uniform B0 { float x; } b0;
        void main(){ gl_Position = vec4(pos,0,1); }
        "#,
        vert
    ).to_vec()
}

fn simple_frag() -> Vec<u32> {
    inline_spirv!(
        r#"
        #version 450
        layout(set=0, binding=1) uniform sampler2D tex;
        layout(location=0) out vec4 o;
        void main(){ o = texture(tex, vec2(0.5)); }
        "#,
        frag
    ).to_vec()
}

fn write_temp_spv(data: &[u32], name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("{}_{}.spv", name, std::process::id()));
    fs::write(&path, bytemuck::cast_slice(data)).unwrap();
    path
}

#[test]
#[serial]
fn from_yaml_builds_pipeline() {
    let mut ctx = make_ctx();
    let rp = RenderPassBuilder::new("rp", Viewport::default())
        .add_subpass(&[AttachmentDescription::default()], None, &[])
        .build(&mut ctx)
        .unwrap();

    let vert = simple_vert();
    let frag = simple_frag();
    let vert_path = write_temp_spv(&vert, "vert");
    let frag_path = write_temp_spv(&frag, "frag");

    let yaml_src = format!(
        "name: yaml_test\nshaders:\n  vertex: {}\n  fragment: {}\n",
        vert_path.display(),
        frag_path.display()
    );
    let mapping: serde_yaml::Mapping = serde_yaml::from_str(&yaml_src).unwrap();
    let mut res = ResourceManager::default();
    let mat = MaterialPipeline::from_yaml(&mut ctx, &mut res, &mapping, rp, 0).unwrap();

    assert!(mat.pipeline.valid());
    assert!(mat.layout.valid());
    assert_eq!(mat.bind_map.get("b0"), Some(&0));
    assert_eq!(mat.bind_map.get("tex"), Some(&1));

    fs::remove_file(vert_path).unwrap();
    fs::remove_file(frag_path).unwrap();
    ctx.destroy();
}

#[test]
#[serial]
#[should_panic(expected = "Missing 'shaders' section")]
fn from_yaml_missing_shaders_panics() {
    let mut ctx = make_ctx();
    let rp = RenderPassBuilder::new("rp", Viewport::default())
        .add_subpass(&[AttachmentDescription::default()], None, &[])
        .build(&mut ctx)
        .unwrap();
    let mapping: serde_yaml::Mapping = serde_yaml::from_str("name: bad").unwrap();
    let mut res = ResourceManager::default();
    let _ = MaterialPipeline::from_yaml(&mut ctx, &mut res, &mapping, rp, 0);
}

#[test]
#[serial]
#[should_panic(expected = "Unknown shader stage")]
fn from_yaml_unknown_stage_panics() {
    let mut ctx = make_ctx();
    let rp = RenderPassBuilder::new("rp", Viewport::default())
        .add_subpass(&[AttachmentDescription::default()], None, &[])
        .build(&mut ctx)
        .unwrap();
    let vert = simple_vert();
    let vert_path = write_temp_spv(&vert, "vert_unknown");
    let yaml_src = format!(
        "name: bad\nshaders:\n  foo: {}\n  vertex: {}\n",
        vert_path.display(),
        vert_path.display()
    );
    let mapping: serde_yaml::Mapping = serde_yaml::from_str(&yaml_src).unwrap();
    let mut res = ResourceManager::default();
    let _ = MaterialPipeline::from_yaml(&mut ctx, &mut res, &mapping, rp, 0);
}
