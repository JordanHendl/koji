#version 450
layout(location = 0) in vec3 vWorldPos;
layout(location = 1) in vec3 vNormal;

layout(set = 0, binding = 1) uniform sampler2DShadow shadowMap;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 lightDir = normalize(vec3(-0.5, -1.0, -0.3));
    vec3 normal = normalize(vNormal);
    float NdotL = max(dot(normal, -lightDir), 0.0);

    // Sample the shadow map
    float shadow = texture(shadowMap, vec3(vWorldPos.xy * 0.5 + 0.5, vWorldPos.z));

    // Combine lighting with shadow factor
    vec3 litColor = vec3(1.0, 1.0, 0.8) * NdotL * shadow;
    outColor = vec4(litColor, 1.0);
}
