#version 450

layout(location = 0) in vec3 vWorldPos;
layout(location = 1) in vec3 vNormal;
layout(location = 2) in vec2 vUV;

layout(set = 0, binding = 0) uniform sampler2D albedo_map;
layout(set = 0, binding = 1) uniform sampler2D normal_map;
layout(set = 0, binding = 2) uniform sampler2D metallic_map;
layout(set = 0, binding = 3) uniform sampler2D roughness_map;

layout(set = 0, binding = 4) uniform CameraBlock {
    mat4 view_proj;
    vec3 cam_pos;
} Camera;

struct Light {
    vec3 position;
    float intensity;
};

layout(set = 0, binding = 5) uniform SceneLightBlock {
    Light light;
} SceneLight;

layout(location = 0) out vec4 outColor;

vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

float distributionGGX(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (3.141592 * denom * denom);
}

float geometrySchlickGGX(float NdotV, float roughness) {
    float r = roughness + 1.0;
    float k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

float geometrySmith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    return geometrySchlickGGX(NdotV, roughness) * geometrySchlickGGX(NdotL, roughness);
}

void main() {
    vec3 albedo = pow(texture(albedo_map, vUV).rgb, vec3(2.2));
    float metallic = texture(metallic_map, vUV).r;
    float roughness = texture(roughness_map, vUV).r;

    // Compute tangent and bitangent from screen-space derivatives
    vec3 dp1 = dFdx(vWorldPos);
    vec3 dp2 = dFdy(vWorldPos);
    vec2 duv1 = dFdx(vUV);
    vec2 duv2 = dFdy(vUV);

    vec3 tangent = normalize(duv2.y * dp1 - duv1.y * dp2);
    vec3 bitangent = normalize(-duv2.x * dp1 + duv1.x * dp2);
    vec3 normal = normalize(cross(tangent, bitangent)); // fallback if needed

    mat3 TBN = mat3(tangent, bitangent, normal);
    vec3 normalSample = texture(normal_map, vUV).xyz * 2.0 - 1.0;
    vec3 N = normalize(TBN * normalSample);

    vec3 V = normalize(Camera.cam_pos - vWorldPos);
    vec3 L = normalize(SceneLight.light.position - vWorldPos);
    vec3 H = normalize(V + L);

    vec3 F0 = mix(vec3(0.04), albedo, metallic);
    vec3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);

    float NDF = distributionGGX(N, H, roughness);
    float G = geometrySmith(N, V, L, roughness);

    float NdotL = max(dot(N, L), 0.0);
    float NdotV = max(dot(N, V), 0.0);

    vec3 numerator = NDF * G * F;
    float denom = max(4.0 * NdotV * NdotL, 0.01); // prevent division artifacts
    vec3 specular = numerator / denom;

    vec3 kS = F;
    vec3 kD = (1.0 - kS) * (1.0 - metallic);

    vec3 diffuse = kD * albedo / 3.141592;
    vec3 lighting = (diffuse + specular) * NdotL * SceneLight.light.intensity;

    // Add ambient term
    vec3 ambient = 0.03 * albedo;

    vec3 color = ambient + lighting;
    color = pow(color, vec3(1.0 / 2.2)); // gamma correction

    outColor = vec4(color, 1.0);
}

