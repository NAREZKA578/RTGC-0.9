#version 330 core

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoords;
in vec3 ViewDir;
in vec3 Tangent;
in vec3 Bitangent;
in vec4 SplatWeights;

out vec4 FragColor;

uniform sampler2D u_texture;
uniform sampler2D u_splatmap;
uniform int u_use_texture;
uniform vec3 u_color;
uniform vec3 u_light_dir;
uniform vec3 u_ambient;
uniform vec3 u_diffuse;
uniform vec3 u_specular;
uniform float u_shininess;
uniform vec3 u_sky_color_top;
uniform vec3 u_sky_color_horizon;
uniform vec3 u_sun_direction;
uniform float u_ambient_intensity;

void main() {
    // Normalize inputs
    vec3 norm = normalize(Normal);
    vec3 viewDir = normalize(ViewDir);

    // Ambient
    vec3 ambient = u_ambient_intensity * u_ambient;

    // Diffuse
    vec3 lightDir = normalize(-u_light_dir);
    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = diff * u_diffuse;

    // Specular
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), u_shininess);
    vec3 specular = spec * u_specular;

    // Texture color with splatmap blending
    vec3 texColor = u_color;
    if (u_use_texture == 1) {
        vec4 splat = texture(u_splatmap, TexCoords);
        
        // Sample individual terrain textures
        vec3 grass = texture(u_texture, TexCoords).rgb;
        vec3 rock = vec3(0.5, 0.5, 0.5);
        vec3 sand = vec3(0.76, 0.7, 0.5);
        vec3 snow = vec3(0.9, 0.9, 0.95);
        
        // Blend based on splatmap weights
        texColor = grass * splat.r + rock * splat.g + sand * splat.b + snow * splat.a;
    }

    // Final color
    vec3 result = (ambient + diffuse + specular) * texColor;

    // Sky gradient based on normal (simple approximation for terrain)
    float heightFactor = norm.y * 0.5 + 0.5;
    vec3 skyColor = mix(u_sky_color_horizon, u_sky_color_top, heightFactor);

    // Blend with sky color based on view angle (simple fog-like effect)
    float viewDot = dot(normalize(FragPos), viewDir);
    float fogFactor = clamp(1.0 - viewDot, 0.0, 0.5);
    result = mix(result, skyColor, fogFactor);

    FragColor = vec4(result, 1.0);
}
