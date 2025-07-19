use koji::canvas::{CanvasDesc, to_yaml, from_yaml, to_json, from_json};

#[test]
fn canvas_yaml_roundtrip() {
    let desc = CanvasDesc { attachments: vec!["color".into(), "depth".into()] };
    let yaml = to_yaml(&desc).unwrap();
    let loaded = from_yaml(&yaml).unwrap();
    assert_eq!(desc.attachments, loaded.attachments);
}

#[test]
fn canvas_json_roundtrip() {
    let desc = CanvasDesc { attachments: vec!["color".into()] };
    let json = to_json(&desc).unwrap();
    let loaded = from_json(&json).unwrap();
    assert_eq!(desc.attachments, loaded.attachments);
}
