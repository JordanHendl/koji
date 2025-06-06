#version 450
layout(location = 0) in vec3 inPos;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec4 inTangent;
layout(location = 3) in vec2 inUV;
layout(location = 4) in vec4 inColor;
layout(location = 5) in uvec4 inJoints;
layout(location = 6) in vec4 inWeights;

layout(set = 0, binding = 0) readonly buffer Bones {
    mat4 bones[];
} bone_buf;

layout(location = 0) out vec4 vColor;

void main() {
    mat4 skin =
        inWeights.x * bone_buf.bones[inJoints.x] +
        inWeights.y * bone_buf.bones[inJoints.y] +
        inWeights.z * bone_buf.bones[inJoints.z] +
        inWeights.w * bone_buf.bones[inJoints.w];
    gl_Position = skin * vec4(inPos, 1.0);
    vColor = inColor;
}
