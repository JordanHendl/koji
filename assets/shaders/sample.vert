#version 450
layout(location = 0) in vec2 inPosition;
layout(location = 0) out vec2 uv;
void main() {
    uv = inPosition * 0.5 + vec2(0.5, 0.5);
    gl_Position = vec4(inPosition, 0.0, 1.0);
}
