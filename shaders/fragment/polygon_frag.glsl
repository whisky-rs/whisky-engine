#version 450

#extension GL_EXT_nonuniform_qualifier : enable

layout(location = 1) in vec3 color;
layout(location = 2) in float radius;
layout(location = 3) in float dist;

layout(location = 0) out vec4 f_color;

void main() {

    // float opacity = smoothstep(radius - 0.005, radius, dist);
    f_color = vec4(color, 1.0);

}