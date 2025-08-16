#define KOJI_MAX_CAMERAS 4
struct Camera {
    mat4 view_proj;
    vec4 cam_pos;
};
layout(set = 0, binding = 4) uniform CameraBuffer {
    Camera cameras[KOJI_MAX_CAMERAS];
} KOJI_cameras;
