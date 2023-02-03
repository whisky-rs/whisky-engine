#version 450

layout(location = 0) in vec2 position;
layout(location = 2) in float radius;
layout(location = 3) in float dist;
layout(location = 4) in vec3 color;

layout(location = 1) out vec3 out_color;
layout(location = 2) out float out_radius;
layout(location = 3) out float out_dist;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    out_color = color;
    out_radius = radius;
    out_dist = dist;
}