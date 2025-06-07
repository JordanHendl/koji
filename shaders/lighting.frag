#version 450

struct Light {
    vec3 position;
    float intensity;
    vec3 color;
    uint _pad;
};

layout(set = 0, binding = 0) uniform sampler2D albedoTex;
layout(set = 0, binding = 1) uniform sampler2D normalTex;
layout(set = 0, binding = 2) buffer Lights { Light lights[]; };
layout(set = 0, binding = 3) uniform LightCount { uint count; };

layout(location = 0) out vec4 outColor;

void main() {
    vec2 uv = gl_FragCoord.xy / vec2(textureSize(albedoTex, 0));
    vec3 albedo = texture(albedoTex, uv).rgb;
    vec3 normal = normalize(texture(normalTex, uv).xyz * 2.0 - 1.0);

    vec3 view_dir = vec3(0.0, 0.0, 1.0);
    vec3 result = vec3(0.0);
    for (uint i = 0u; i < count; ++i) {
        vec3 light_dir = normalize(lights[i].position);
        float diff = max(dot(normal, light_dir), 0.0);
        vec3 half_dir = normalize(light_dir + view_dir);
        float spec = pow(max(dot(normal, half_dir), 0.0), 16.0);
        vec3 light_color = lights[i].color * lights[i].intensity;
        result += albedo * diff * light_color + spec * light_color;
    }
    outColor = vec4(result, 1.0);
}
