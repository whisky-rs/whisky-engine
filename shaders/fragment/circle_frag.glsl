#version 450

layout(location = 0) in flat uint tex_i;
layout(location = 1) in flat float radius;
layout(location = 2) in flat vec2 center;
layout(location = 3) in vec2 position;
layout(location = 4) in vec3 color;


layout(location = 0) out vec4 f_color;

void main() {
    float opacity = smoothstep(radius - 0.03, radius - 0.025, distance(position, center))
    * (1.0 - smoothstep(radius - 0.01, radius , distance(position, center)));
    f_color = vec4(vec3(color), opacity);

}