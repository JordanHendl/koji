#version 450
layout(set = 0, binding = 4) uniform Camera {
    mat4 view_proj;
    vec3 cam_pos;
};

layout(location = 0) in vec3 inPos;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec4 inTangent;
layout(location = 3) in vec2 inUV;
layout(location = 4) in vec4 inColor;

layout(location = 0) out vec3 vWorldPos;
layout(location = 1) out vec3 vNormal;
layout(location = 2) out vec2 vUV;

void main() {
    mat4 model = mat4(1.0);
    vec4 world = model * vec4(inPos, 1.0);
    vWorldPos = world.xyz;
    vNormal = mat3(model) * inNormal;
    vUV = inUV;
    gl_Position = view_proj * world;
}
