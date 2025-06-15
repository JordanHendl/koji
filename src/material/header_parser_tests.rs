use super::*;
use std::collections::HashSet;

#[test]
fn parse_hlsl_headers() {
    let hlsl = "Texture2D gTex;\nfloat t = current_time_ms();";
    let result = parse_headers(hlsl);
    let expected: HashSet<String> = ["time", "texture"].iter().map(|s| s.to_string()).collect();
    assert_eq!(result, expected);
}

#[test]
fn parse_slang_headers() {
    let slang = "float current_time_ms();\n// koji:debug";
    let result = parse_headers(slang);
    let expected: HashSet<String> = ["time", "debug"].iter().map(|s| s.to_string()).collect();
    assert_eq!(result, expected);
}
