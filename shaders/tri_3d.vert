#version 450

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 norm;
layout (location = 2) in vec4 color;

layout (location = 0) out vec3 o_norm;
layout (location = 1) out vec4 o_color;

layout (push_constant) uniform Input {
  vec2 window_dims;
} inputs;

void main() {
  gl_Position = vec4(pos, 1.0);
  o_norm = norm;
  o_color = color;
}
