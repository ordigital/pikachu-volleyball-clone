#version 100
precision mediump float;
varying mediump vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
uniform vec2 OutlineUv;
float maskAt(vec2 point) {
    if (point.x < 0.0 || point.y < 0.0 || point.x > 1.0 || point.y > 1.0) return 0.0;
    return texture2D(Texture, point).r;
}
void main() {
    float fill = maskAt(uv);
    float coverage = fill;
    for (int i = 0; i < 16; i++) {
        float angle = float(i) * 6.28318530718 / 16.0;
        coverage = max(coverage, maskAt(uv + vec2(cos(angle), sin(angle)) * OutlineUv));
    }
    // Jeden przebieg i rzeczywista maska obrysu zamiast 16 przesuniętych napisów.
    gl_FragColor = vec4(color.rgb * fill / max(coverage, 0.0001), coverage * color.a);
}
