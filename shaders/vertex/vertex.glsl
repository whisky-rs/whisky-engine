#version 450

layout(location = 0) in vec2 position;
layout(location = 5) in vec2 tex_position;
layout(location = 6) in vec3 color;
layout(location = 0) out vec2 v_tex_position;
layout(location = 1) out vec4 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_tex_position = tex_position;
    v_color = vec4(color, 1.0);
}