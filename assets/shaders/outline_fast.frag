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
    // 8 próbek na okręgu zamiast 16 – lekki shader dla GPU urządzeń mobilnych
    coverage = max(coverage, maskAt(uv + vec2(OutlineUv.x, 0.0)));
    coverage = max(coverage, maskAt(uv - vec2(OutlineUv.x, 0.0)));
    coverage = max(coverage, maskAt(uv + vec2(0.0, OutlineUv.y)));
    coverage = max(coverage, maskAt(uv - vec2(0.0, OutlineUv.y)));
    coverage = max(coverage, maskAt(uv + OutlineUv * 0.7071));
    coverage = max(coverage, maskAt(uv - OutlineUv * 0.7071));
    coverage = max(coverage, maskAt(uv + vec2(OutlineUv.x, -OutlineUv.y) * 0.7071));
    coverage = max(coverage, maskAt(uv + vec2(-OutlineUv.x, OutlineUv.y) * 0.7071));

    gl_FragColor = vec4(color.rgb * fill / max(coverage, 0.0001), coverage * color.a);
}
