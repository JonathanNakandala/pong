#version 450

layout(location = 3) in vec2 position;
//layout(location = 2) in vec2 offset;
layout(location = 4) out vec3 fragColor;
vec3 colorsss[6] = vec3[](
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(0.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(0.0, 1.0, 1.0)
);
void main() {
    //vec4 totalOffset = vec4(offset.x, offset.y, 0.0, 0.0);
    gl_Position = vec4(position, 0.0, 1.0);
    fragColor = colorsss[gl_VertexIndex];
}