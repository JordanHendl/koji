#version 450
#extension GL_EXT_nonuniform_qualifier : enable
layout(location = 0) in vec2 vUV;
layout(location = 1) flat in uint vTexIndex;
layout(set = 0, binding = 0) uniform sampler2D glyph_textures[];
layout(location = 0) out vec4 outColor;
void main() {
    outColor = texture(glyph_textures[vTexIndex], vUV);
}
