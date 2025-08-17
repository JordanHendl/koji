#version 450
#include "../../assets/shaders/camera.slang"
layout(location=0) out vec4 o;
void main(){ o = KOJI_cameras.cameras[0].cam_pos; }
