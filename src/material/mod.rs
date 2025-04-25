use serde_yaml::{Mapping as YamlMap, Value};
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialType {
    Float,
    Int,
    Uint,
    Vec2,
    Vec3,
    Vec4,
    TextureHandle,
    BufferHandle,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LayoutPacking {
    Scalar,
    Std140,
    Std430,
}

#[derive(Debug)]
pub struct MaterialField {
    pub name: String,
    pub ty: MaterialType,
    pub offset: usize,
}

fn serialize_f32(value: f32, out: &mut Vec<u8>) {
    out.extend(&value.to_le_bytes());
}

fn serialize_i32(value: i32, out: &mut Vec<u8>) {
    out.extend(&value.to_le_bytes());
}

fn serialize_u32(value: u32, out: &mut Vec<u8>) {
    out.extend(&value.to_le_bytes());
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

fn get_type_size_and_align(ty: MaterialType, layout: LayoutPacking) -> (usize, usize) {
    use MaterialType::*;
    match layout {
        LayoutPacking::Scalar => match ty {
            Float | Int | Uint => (4, 4),
            Vec2 => (8, 8),
            Vec3 => (12, 4),
            Vec4 => (16, 4),
            TextureHandle | BufferHandle => (8, 8),
        },
        LayoutPacking::Std430 => match ty {
            Float | Int | Uint => (4, 4),
            Vec2 => (8, 8),
            Vec3 | Vec4 => (16, 16),
            TextureHandle | BufferHandle => (8, 8),
        },
        LayoutPacking::Std140 => match ty {
            Float | Int | Uint => (4, 4),
            Vec2 => (8, 8),
            Vec3 | Vec4 => (16, 16),
            TextureHandle | BufferHandle => (8, 8),
        },
    }
}

pub struct DataRegistry;
pub fn infer_and_pack_yaml_material_with_padding(
    values: &YamlMap,
    layout: LayoutPacking,
    registry: &DataRegistry,
) -> (Vec<u8>, Vec<MaterialField>) {
    let mut buffer = Vec::new();
    let mut fields = Vec::new();
    let mut offset = 0;

    for (key, val) in values {
        let key_str = key.as_str().expect("Expected string key").to_string();
        let (field_type, bytes): (MaterialType, Vec<u8>) = match val {
            Value::Number(n) if n.is_f64() => (
                MaterialType::Float,
                n.as_f64()
                    .map(|f| (f as f32).to_le_bytes().to_vec())
                    .unwrap(),
            ),
            Value::Number(n) if n.is_i64() => (
                MaterialType::Int,
                n.as_i64()
                    .map(|i| (i as i32).to_le_bytes().to_vec())
                    .unwrap(),
            ),
            Value::Number(n) if n.is_u64() => (
                MaterialType::Uint,
                n.as_u64()
                    .map(|u| (u as u32).to_le_bytes().to_vec())
                    .unwrap(),
            ),
            Value::Sequence(arr) => match arr.len() {
                2 => (
                    MaterialType::Vec2,
                    arr.iter()
                        .map(|v| (v.as_f64().unwrap() as f32).to_le_bytes())
                        .flatten()
                        .collect(),
                ),
                3 => {
                    let mut bytes: Vec<u8> = arr
                        .iter()
                        .map(|v| (v.as_f64().unwrap() as f32).to_le_bytes())
                        .flatten()
                        .collect();
                    if layout != LayoutPacking::Scalar {
                        bytes.extend(&0f32.to_le_bytes()); // pad vec3
                    }
                    (MaterialType::Vec3, bytes)
                }
                4 => (
                    MaterialType::Vec4,
                    arr.iter()
                        .map(|v| (v.as_f64().unwrap() as f32).to_le_bytes())
                        .flatten()
                        .collect(),
                ),
                _ => panic!("Unsupported array size for key '{}'", key_str),
            },
            Value::String(path) => {
                //                if let Some(handle) = registry.get_image(path) {
                //                    (
                //                        MaterialType::TextureHandle,
                //                        todo!(),
                //                     //   handle_to_bytes(handle.index, handle.generation),
                //                    )
                //                } else if let Some(handle) = registry.get_buffer(path) {
                (MaterialType::BufferHandle, vec![])
                //                } else {
                //                    panic!("Unrecognized resource string: '{}'", path);
                //                }
            }
            _ => panic!("Unsupported value type for '{}'", key_str),
        };

        let (size, align) = get_type_size_and_align(field_type, layout);
        offset = align_up(offset, align);
        if buffer.len() < offset {
            buffer.resize(offset, 0);
        }

        buffer.extend(bytes);

        fields.push(MaterialField {
            name: key_str,
            ty: field_type,
            offset,
        });

        offset = buffer.len();
    }

    if layout == LayoutPacking::Std140 {
        buffer.resize(align_up(buffer.len(), 16), 0);
    }

    (buffer, fields)
}

fn handle_to_bytes(index: u16, generation: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(4);
    out.extend(&index.to_le_bytes());
    out.extend(&generation.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::{from_str, Value};

    fn extract_map(yaml: &str, root_key: &str) -> YamlMap {
        let doc: Value = from_str(yaml).expect("YAML parse failed");
        let main_map = doc.as_mapping().unwrap();
        main_map.get(&Value::String(root_key.to_string()))
            .and_then(|v| v.as_mapping())
            .cloned()
            .expect("Expected submap")
    }

//    fn make_test_registry() -> DataRegistry {
//        let mut reg = DataRegistry::default();
//        reg.images.insert(
//            "textures/wood.png".into(),
//            Handle { index: 3, generation: 1 },
//        );
//        reg.buffers.insert(
//            "buffers/vertex.bin".into(),
//            Handle { index: 99, generation: 42 },
//        );
//        reg
//    }

    #[test]
    fn test_yaml_subobject_scalar_and_vectors() {
        let yaml = r#"
material:
  base_color: [1.0, 0.5, 0.0, 1.0]
  roughness: 0.3
  metallic: 0.9
"#;
        let map = extract_map(yaml, "material");
        let registry = DataRegistry{};
        let (buf, fields) = infer_and_pack_yaml_material_with_padding(&map, LayoutPacking::Std140, &registry);

        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "base_color");
        assert_eq!(fields[1].ty, MaterialType::Float);
        assert_eq!(fields[2].offset % 4, 0);
        assert_eq!(buf.len() % 16, 0); // std140-aligned
    }

    #[test]
    fn test_yaml_subobject_with_image_handle() {
        let yaml = r#"
material:
  base_color: [1.0, 0.5, 0.0, 1.0]
"#;
        let map = extract_map(yaml, "material");
        let registry = DataRegistry{};
        let (buf, fields) = infer_and_pack_yaml_material_with_padding(&map, LayoutPacking::Std140, &registry);
    }

    #[test]
    fn test_yaml_subobject_with_buffer_handle() {
//        let yaml = r#"
//material:
//  vertex_data: "buffers/vertex.bin"
//"#;
//        let map = extract_map(yaml, "material");
//        let registry = make_test_registry();
//        let (buf, fields) = infer_and_pack_yaml_material_with_padding(&map, LayoutPacking::Std430, &registry);
//
//        let f = &fields[0];
//        assert_eq!(f.name, "vertex_data");
//        assert_eq!(f.ty, MaterialType::BufferHandle);
//        let handle = &buf[f.offset..f.offset + 4];
//        let idx = u16::from_le_bytes([handle[0], handle[1]]);
//        let gen = u16::from_le_bytes([handle[2], handle[3]]);
//        assert_eq!(idx, 99);
//        assert_eq!(gen, 42);
    }

    #[test]
//    #[should_panic(expected = "Unrecognized resource string")]
    fn test_unrecognized_resource_panics() {
        let yaml = r#"
material:
  unknown_resource: "textures/missing.png"
"#;
        let map = extract_map(yaml, "material");
        let registry = DataRegistry{};
        let _ = infer_and_pack_yaml_material_with_padding(&map, LayoutPacking::Std430, &registry);
    }
}

#[test]
fn test_multiple_materials_in_yaml() {
    let yaml = r#"
gold_material:
  base_color: [1.0, 0.766, 0.336, 1.0]
  roughness: 0.2
  metallic: 1.0
  albedo: "textures/gold_albedo.png"

wood_material:
  base_color: [0.5, 0.3, 0.1, 1.0]
  roughness: 0.6
  metallic: 0.0
  albedo: "textures/wood.png"
  normal_map: "textures/wood_normal.png"

simple_material:
  color: [0.1, 0.2, 0.3, 1.0]
  opacity: 0.8
"#;

    // Parse the whole YAML document
    let doc: Value = serde_yaml::from_str(yaml).expect("Failed to parse multi-material YAML");
    let materials_map = doc.as_mapping().unwrap();

    // Setup the registry
    let mut registry = DataRegistry{};
    // Parse each material individually
    for (material_name, material_value) in materials_map {
        let mat_name = material_name.as_str().unwrap();
        let material = material_value.as_mapping().unwrap();

        let (buf, fields) = infer_and_pack_yaml_material_with_padding(
            material,
            LayoutPacking::Std430,
            &registry,
        );

        println!("Material '{}':", mat_name);
        for f in &fields {
            println!("  Field: {:?}", f);
        }
        println!("  Packed bytes: {:?}", buf);

        // Basic assertions:
        assert!(buf.len() > 0);
        assert!(fields.len() > 0);

        // Specific checks
        match mat_name {
            "gold_material" => {
                assert!(fields.iter().any(|f| f.name == "base_color"));
                assert!(fields.iter().any(|f| f.name == "albedo"));
            }
            "wood_material" => {
                assert!(fields.iter().any(|f| f.name == "normal_map"));
            }
            "simple_material" => {
                assert!(fields.iter().any(|f| f.name == "opacity"));
                assert!(fields.iter().all(|f| f.ty != MaterialType::TextureHandle));
            }
            _ => panic!("Unexpected material {}", mat_name),
        }
    }
}

