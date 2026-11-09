#version 100
precision highp float;
varying mediump vec2 uv;
varying lowp vec4 color;
varying highp vec2 world;
uniform sampler2D Texture;
uniform vec4 CloudRects[10];
uniform float ReceiverIndex;
float cloudMask(vec2 point, vec4 rect) {
    if (rect.z <= 0.0 || rect.w <= 0.0) return 0.0;
    vec2 sampleUv = (point - rect.xy) / rect.zw;
    if (sampleUv.x < 0.0 || sampleUv.y < 0.0 || sampleUv.x > 1.0 || sampleUv.y > 1.0) return 0.0;
    return texture2D(Texture, sampleUv).a;
}
void main() {
    vec4 receiver = texture2D(Texture, uv) * color;
    float shadow = 0.0;
    for (int i = 0; i < 10; i++) {
        if (float(i) > ReceiverIndex) {
            vec2 point = world - vec2(3.0, 3.0);
            vec4 caster = CloudRects[i];
            if (caster.z <= 0.0 || caster.w <= 0.0 ||
                point.x < caster.x - 2.0 || point.y < caster.y - 2.0 ||
                point.x > caster.x + caster.z + 2.0 || point.y > caster.y + caster.w + 2.0) continue;
            float blurred = cloudMask(point, CloudRects[i]) * 0.28;
            blurred += cloudMask(point + vec2(2.0, 0.0), CloudRects[i]) * 0.12;
            blurred += cloudMask(point - vec2(2.0, 0.0), CloudRects[i]) * 0.12;
            blurred += cloudMask(point + vec2(0.0, 2.0), CloudRects[i]) * 0.12;
            blurred += cloudMask(point - vec2(0.0, 2.0), CloudRects[i]) * 0.12;
            blurred += cloudMask(point + vec2(1.5, 1.5), CloudRects[i]) * 0.06;
            blurred += cloudMask(point - vec2(1.5, 1.5), CloudRects[i]) * 0.06;
            blurred += cloudMask(point + vec2(-1.5, 1.5), CloudRects[i]) * 0.06;
            blurred += cloudMask(point + vec2(1.5, -1.5), CloudRects[i]) * 0.06;
            shadow += blurred * 0.17;
        }
    }
    // Zachowujemy dokładnie alfa odbiorcy: cień nigdy nie trafi na tło.
    gl_FragColor = vec4(receiver.rgb * (1.0 - min(shadow, 0.26)), receiver.a);
}
