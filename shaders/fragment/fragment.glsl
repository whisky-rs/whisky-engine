#version 450

layout(location = 0) in vec2 v_tex_position;
layout(location = 1) in vec4 v_color;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

void main() {
    f_color = v_color * texture(tex, v_tex_position)[0];
}