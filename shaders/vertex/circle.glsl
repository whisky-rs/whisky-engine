#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in uint texture_id;
layout(location = 2) in float radius;
layout(location = 3) in vec2 center;
layout(location = 4) in vec3 color;

layout(location = 0) out flat uint out_tex_i;
layout(location = 1) out flat float out_radius;
layout(location = 2) out flat vec2 out_center;
layout(location = 3) out vec2 out_position;
layout(location = 4) out vec3 out_color;

void main() {
    float pct = 0.0;
    gl_Position = vec4(position, 0.0, 1.0);
    out_tex_i = texture_id;
    out_radius = radius;
    out_center = center;
    out_position = position;
    out_color = color;
}