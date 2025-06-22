#version 450
#include "timing.slang"
layout(set = 0, binding = 1) uniform sampler2D tex;
layout(set = 0, binding = 2) uniform UBO { float v; } ubo;
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 outColor;
void main() {
    float t = KOJI_time.info.currentTimeMs;
    vec3 dynamic = vec3(0.5 + 0.5 * sin(t / 1000.0), 0.5 + 0.5 * cos(t), 0.5);
    vec4 color = texture(tex, uv);
    outColor = vec4(vec3(sin(t)), 1.0);
}
