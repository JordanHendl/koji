#version 450
layout(location = 0) in vec3 inPos;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec4 inTangent;
layout(location = 3) in vec2 inUV;
layout(location = 4) in vec4 inColor;
layout(location = 0) out vec3 vNormal;
layout(location = 1) out vec2 vUV;
void main() {
    vNormal = inNormal;
    vUV = inUV;
    gl_Position = vec4(inPos, 1.0);
}
