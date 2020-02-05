#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 colour;
layout(location = 0) out vec3 fragColor;

void main() {
    //vec4 totalOffset = vec4(offset.x, offset.y, 0.0, 0.0);
    gl_Position = vec4(position, 0.0, 1.0);
    fragColor = colour;
}