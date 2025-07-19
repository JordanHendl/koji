use super::CanvasDesc;

pub fn to_yaml(desc: &CanvasDesc) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(desc)
}

pub fn from_yaml(data: &str) -> Result<CanvasDesc, serde_yaml::Error> {
    serde_yaml::from_str(data)
}

pub fn to_json(desc: &CanvasDesc) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(desc)
}

pub fn from_json(data: &str) -> Result<CanvasDesc, serde_json::Error> {
    serde_json::from_str(data)
}
