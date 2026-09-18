#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;
uniform vec4 colDiffuse;

uniform vec2  resolution;   // 输入纹理尺寸
uniform float radius;       // 模糊半径（像素）
uniform float noiseAmount;  // 磨砂噪点强度，0~0.05 比较自然

out vec4 finalColor;

const float GOLDEN_ANGLE = 2.39996323; // 黄金角，约 137.5°

// 稳定 hash，用于磨砂噪点
float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

void main() {
    vec2 texel = 1.0 / resolution;

    const int SAMPLES = 32;          // 采样数，16~48 都够用
    float sigma  = max(radius, 0.001);
    float sigma2 = 2.0 * sigma * sigma;

    vec3  sum   = texture(texture0, fragTexCoord).rgb; // 中心
    float total = 1.0;

    // 黄金角螺旋：面积均匀，无条带、无方向性伪影
    for (int i = 1; i < SAMPLES; i++) {
        float fi = float(i);
        float r  = sqrt(fi / float(SAMPLES)) * radius;  // 面积均匀分布
        float th = fi * GOLDEN_ANGLE;
        vec2  offset = vec2(cos(th), sin(th)) * r * texel;

        float w = exp(-r * r / sigma2);
        sum   += texture(texture0, fragTexCoord + offset).rgb * w;
        total += w;
    }

    vec3 color = sum / total;

    // 磨砂颗粒：让玻璃有"质感"，不是单纯糊
    float n = hash(fragTexCoord * resolution + vec2(1.0, 17.0)) - 0.5;
    color += n * noiseAmount;

    finalColor = vec4(color, 1.0) * colDiffuse * fragColor;
}
