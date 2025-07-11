#version 450
layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;
};

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;

layout(location = 0) out vec3 worldPos;
layout(location = 1) out vec3 normal;

void main() {
    mat4 model = mat4(1.0);
    vec4 world = model * vec4(inPosition, 1.0);
    worldPos = world.xyz;
    normal = mat3(model) * inNormal;
    gl_Position = view_proj * world;
}
