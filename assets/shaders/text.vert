#version 450
layout(location = 0) in vec3 inPos;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec4 inTangent;
layout(location = 3) in vec2 inUV;
layout(location = 4) in vec4 inColor;
layout(location = 0) out vec2 vUV;
layout(location = 1) out vec4 vColor;
layout(location = 2) flat out uint vTexIndex;
void main() {
    gl_Position = vec4(inPos, 1.0);
    vUV = inUV;
    vColor = inColor;
    vTexIndex = uint(inTangent.w);
}
