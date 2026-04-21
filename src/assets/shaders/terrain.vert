#version 330 core

layout (location = 0) in vec3 a_position;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec2 a_tex_coords;

out vec3 FragPos;
out vec3 Normal;
out vec2 TexCoords;
out vec3 ViewDir;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform vec3 u_camera_pos;

void main() {
    FragPos = vec3(u_model * vec4(a_position, 1.0));
    Normal = mat3(transpose(inverse(u_model))) * a_normal;
    TexCoords = a_tex_coords;
    
    vec4 viewPos = u_view * u_model * vec4(a_position, 1.0);
    ViewDir = normalize(vec3(-viewPos));
    
    gl_Position = u_projection * u_view * u_model * vec4(a_position, 1.0);
}
