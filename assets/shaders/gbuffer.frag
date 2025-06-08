#version 450
layout(location = 0) out vec4 outAlbedo;
layout(location = 1) out vec4 outNormal;
void main() {
    outAlbedo = vec4(1.0, 0.0, 0.0, 1.0);
    outNormal = vec4(0.0, 0.0, 1.0, 1.0);
}
