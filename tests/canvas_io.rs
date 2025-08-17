use dashi::Format;
use koji::canvas::{AttachmentDesc, CanvasDesc, from_json, from_yaml, to_json, to_yaml};

#[test]
fn canvas_yaml_roundtrip() {
    let desc = CanvasDesc {
        extent: [1, 1],
        attachments: vec![
            AttachmentDesc { name: "color".into(), format: Format::RGBA8 },
            AttachmentDesc { name: "depth".into(), format: Format::D24S8 },
        ],
    };
    let yaml = to_yaml(&desc).unwrap();
    let loaded = from_yaml(&yaml).unwrap();
    assert_eq!(desc, loaded);
}

#[test]
fn canvas_json_roundtrip() {
    let desc = CanvasDesc {
        extent: [1, 1],
        attachments: vec![AttachmentDesc { name: "color".into(), format: Format::RGBA8 }],
    };
    let json = to_json(&desc).unwrap();
    let loaded = from_json(&json).unwrap();
    assert_eq!(desc, loaded);
}
