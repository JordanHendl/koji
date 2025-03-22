use super::ShaderModule;

pub struct TimeDate {}

impl TimeDate {
    pub fn new() -> Self {
        Self {}
    }
}
impl ShaderModule for TimeDate {
    fn resource(&self, name: &str) -> Option<dashi::ShaderResource> {
        None
    }

    fn update(&mut self) {
        todo!()
    }
}
