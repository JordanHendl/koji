#version 450

layout(location = 0) in vec3 vNormal;
layout(location = 1) in vec2 vUV;

layout(set = 0, binding = 0) uniform sampler2D albedo_map;
layout(set = 0, binding = 1) uniform sampler2D normal_map;
layout(set = 0, binding = 2) uniform sampler2D metallic_map;
layout(set = 0, binding = 3) uniform sampler2D roughness_map;

layout(location = 0) out vec4 outColor;

const vec3 cameraDir = vec3(0.0, 0.0, 1.0); // front view
const vec3 lightDir = normalize(vec3(0.0, 0.0, 1.0)); // front light

// Fresnel-Schlick approximation
vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

// GGX Normal Distribution Function
float distributionGGX(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (3.141592 * denom * denom);
}

// Smith?s Schlick-GGX Geometry function
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
    vec3 albedo = pow(texture(albedo_map, vUV).rgb, vec3(2.2)); // gamma to linear
    vec3 normal = texture(normal_map, vUV).xyz * 2.0 - 1.0;
    vec3 N = normalize(normal);
    vec3 V = normalize(cameraDir);
    vec3 L = normalize(lightDir);
    vec3 H = normalize(V + L);

    float roughness = texture(roughness_map, vUV).r;
    float metallic = texture(metallic_map, vUV).r;

    // Fresnel reflectance at normal incidence
    vec3 F0 = mix(vec3(0.04), albedo, metallic);
    vec3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);

    float NDF = distributionGGX(N, H, roughness);
    float G = geometrySmith(N, V, L, roughness);

    float NdotL = max(dot(N, L), 0.0);
    float NdotV = max(dot(N, V), 0.0);

    vec3 numerator = NDF * G * F;
    float denom = 4.0 * NdotV * NdotL + 0.001;
    vec3 specular = numerator / denom;

    vec3 kS = F;
    vec3 kD = (1.0 - kS) * (1.0 - metallic);

    vec3 diffuse = kD * albedo / 3.141592;
    vec3 color = (diffuse + specular) * NdotL;

    // final gamma correction
    color = pow(color, vec3(1.0 / 2.2));
    outColor = vec4(color, 1.0);
}

