#version 330 core

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoords;

out vec4 FragColor;

uniform sampler2D u_texture;
uniform int u_use_texture;
uniform vec3 u_color;
uniform vec3 u_light_dir;
uniform vec3 u_ambient;
uniform vec3 u_diffuse;
uniform vec3 u_specular;
uniform float u_shininess;
uniform int u_lights_enabled;
uniform vec3 u_emissive_color;

void main() {
    // Normalize inputs
    vec3 norm = normalize(Normal);
    
    // Ambient
    vec3 ambient = u_ambient;
    
    // Diffuse
    vec3 lightDir = normalize(-u_light_dir);
    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = diff * u_diffuse;
    
    // Specular
    vec3 viewDir = normalize(-FragPos);
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), u_shininess);
    vec3 specular = spec * u_specular;
    
    // Emissive (for vehicle lights)
    vec3 emissive = vec3(0.0);
    if (u_lights_enabled == 1) {
        emissive = u_emissive_color;
    }
    
    // Texture color
    vec3 texColor = u_color;
    if (u_use_texture == 1) {
        texColor = texture(u_texture, TexCoords).rgb;
    }
    
    // Final color
    vec3 result = (ambient + diffuse + specular + emissive) * texColor;
    
    FragColor = vec4(result, 1.0);
}
