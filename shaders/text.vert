#version 450
layout(location = 0) in vec3 inPos;
layout(location = 1) in vec2 inUV;
layout(location = 0) out vec2 vUV;
void main() {
    gl_Position = vec4(inPos, 1.0);
    vUV = inUV;
}
