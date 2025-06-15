use std::collections::HashSet;

/// Scan shader source (e.g. Slang or GLSL) for special Koji directives.
///
/// Lines referencing well known resources will cause their names (e.g. `"time"`)
/// to be returned. The directives are parsed directly from the source code
/// before it is compiled to SPIR-V, as comments are stripped during
/// compilation.
pub fn parse_headers(src: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    for line in src.lines() {
        let trimmed = line.trim();

        // Handle `// koji:name` style comments
        if let Some(pos) = trimmed.find("//") {
            let comment = &trimmed[pos + 2..].trim();
            if let Some(name) = comment.strip_prefix("koji:") {
                let name = name.trim();
                if !name.is_empty() {
                    out.insert(name.to_string());
                }
            }
        }
    }

    // Look for well known resource/function names. If present, add their
    // short names to the result set so the caller knows which resources are
    // required by the shader.
    if src.contains("current_time_ms") || src.contains("gTime") {
        out.insert("time".to_string());
    }

    if src.contains("gTex") || src.contains("gTextures") {
        out.insert("texture".to_string());
    }

    if src.contains("gDebug") {
        out.insert("debug".to_string());
    }

    out
}

