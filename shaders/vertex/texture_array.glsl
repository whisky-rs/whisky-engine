#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in uint texture_id;
layout(location = 6) in vec2 tex_position;
layout(location = 0) out vec2 out_tex_coords;
layout(location = 1) out uint layer;

// const float x[4] = float[](0.0, 0.0, 1.0, 1.0);
// const float y[4] = float[](0.0, 1.0, 0.0, 1.0);

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    out_tex_coords = tex_position;
    layer = texture_id;
}