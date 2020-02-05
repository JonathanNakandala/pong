#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 colour;
layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform Displacement {
    float displacement;
} disp;

void main() {
    gl_Position = vec4(position.x,position.y+disp.displacement, 0.0, 1.0);
    fragColor = colour;
}