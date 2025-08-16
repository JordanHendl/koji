#version 450
#include "camera.slang"

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;

layout(location = 0) out vec3 worldPos;
layout(location = 1) out vec3 normal;

void main() {
    mat4 model = mat4(1.0);
    vec4 world = model * vec4(inPosition, 1.0);
    worldPos = world.xyz;
    normal = mat3(model) * inNormal;
    gl_Position = KOJI_cameras[0].view_proj * world;
}
