#version 450
layout(location = 0) in vec3 vNormal;
layout(location = 1) in vec2 vUV;
layout(set = 0, binding = 0) uniform sampler2D albedo_map;
layout(set = 0, binding = 1) uniform sampler2D normal_map;
layout(set = 0, binding = 2) uniform sampler2D metallic_map;
layout(set = 0, binding = 3) uniform sampler2D roughness_map;
layout(location = 0) out vec4 outColor;
void main() {
    vec3 albedo = texture(albedo_map, vUV).rgb;
    vec3 normal = texture(normal_map, vUV).xyz * 2.0 - 1.0;
    float metallic = texture(metallic_map, vUV).r;
    float roughness = texture(roughness_map, vUV).r;
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));
    vec3 n = normalize(normal);
    float NdotL = max(dot(n, lightDir), 0.0);
    vec3 color = albedo * NdotL;
    vec3 specColor = mix(vec3(0.04), albedo, metallic);
    float spec = pow(max(dot(reflect(-lightDir, n), vec3(0.0, 0.0, 1.0)), 0.0), 1.0/(roughness*roughness+0.0001));
    color += specColor * spec;
    outColor = vec4(color, 1.0);
}
