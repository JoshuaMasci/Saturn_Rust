#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 1) in vec4 in_color;

layout(location = 0) out vec4 out_color;

layout(binding = 0) uniform sampler2D texture_atlas;

void main()
{
    out_color = in_color * vec4(1.0, 1.0, 1.0, texture(texture_atlas, in_uv).r);
}