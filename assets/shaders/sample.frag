#version 450
#include "timing.slang"
layout(set = 0, binding = 0) uniform sampler2D tex;
layout(set = 0, binding = 1) uniform UBO { float v; } ubo;
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 outColor;
void main() {
    vec4 color = texture(tex, uv);
    outColor = mix(color, vec4(0.0, 1.0, 0.0, 1.0), ubo.v);
}
