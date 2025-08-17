#define KOJI_MAX_CAMERAS 4

struct Camera {
#if defined(__HLSL__) || defined(__SLANG__)
    float4x4 view_proj;
    float4 cam_pos;
#else
    mat4 view_proj;
    vec4 cam_pos;
#endif
};

#if defined(__HLSL__) || defined(__SLANG__)
struct CameraBuffer { Camera cameras[KOJI_MAX_CAMERAS]; };
[[vk::binding(4, 0)]] ConstantBuffer<CameraBuffer> KOJI_cameras;
#else
layout(set = 0, binding = 4) uniform CameraBuffer { Camera cameras[KOJI_MAX_CAMERAS]; } KOJI_cameras;
#endif
