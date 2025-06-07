#version 450
layout(location = 0) in vec2 vUV;
layout(set = 0, binding = 0) uniform sampler2D glyph_tex;
layout(location = 0) out vec4 outColor;
void main() {
    outColor = texture(glyph_tex, vUV);
}
