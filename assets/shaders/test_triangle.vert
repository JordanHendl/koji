#version 450
layout(location = 0) in vec3 inPos;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec4 inTangent;
layout(location = 3) in vec2 inUV;
layout(location = 4) in vec4 inColor;
layout(location = 0) out vec4 vColor;
void main() {
    vColor = inColor;
    gl_Position = vec4(inPos, 1.0);
}
