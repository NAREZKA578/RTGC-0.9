#version 330 core

in vec4 v_color;
in vec2 v_uv;
out vec4 FragColor;

uniform sampler2D u_texture;
uniform bool u_use_texture;

void main() {
    if (u_use_texture) {
        vec4 tex_color = texture(u_texture, v_uv);
        FragColor = vec4(v_color.rgb * tex_color.rgb, v_color.a * tex_color.a);
    } else {
        FragColor = v_color;
    }
}
