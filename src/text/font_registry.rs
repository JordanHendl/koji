use std::collections::HashMap;
use rusttype::Font;

/// Registry storing parsed fonts for reuse.
pub struct FontRegistry {
    fonts: HashMap<String, Font<'static>>,
}

impl FontRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { fonts: HashMap::new() }
    }

    /// Parse and store font data under `name`.
    pub fn register_font(&mut self, name: &str, bytes: &[u8]) {
        if let Some(font) = Font::try_from_vec(bytes.to_vec()) {
            self.fonts.insert(name.to_string(), font);
        } else {
            panic!("invalid font data for {}", name);
        }
    }

    /// Retrieve a cloned [`Font`] by name.
    pub fn get(&self, name: &str) -> Option<Font<'static>> {
        self.fonts.get(name).cloned()
    }
}
