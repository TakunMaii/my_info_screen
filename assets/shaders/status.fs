#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;
uniform vec4 colDiffuse;
uniform float time;

out vec4 finalColor;

void main() {
    float breathing = 0.975 + 0.025 * sin(time * 1.8);
    vec4 texelColor = texture(texture0, fragTexCoord);
    finalColor = vec4(texelColor.rgb * fragColor.rgb * breathing, texelColor.a * fragColor.a) * colDiffuse;
}
