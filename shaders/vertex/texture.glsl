#version 450

layout(location = 0) in vec2 position;
layout(location = 0) out vec2 tex_coords;

const float x[4] = float[](0.0, 0.0, 1.0, 1.0);
const float y[4] = float[](0.0, 1.0, 0.0, 1.0);

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    tex_coords = vec2(x[int(mod( gl_VertexIndex, 4))], y[int(mod( gl_VertexIndex, 4))] );
}