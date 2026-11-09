#version 100
precision highp float;
varying mediump vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
uniform float Time;
uniform vec2 UvScale;
void main() {
    vec2 point = uv * UvScale + vec2(0.012, 0.008) * Time;
    // Lustrzane kafelkowanie zachowuje ciągłość na niepasujących brzegach tekstury.
    vec2 sampleUv = 1.0 - abs(mod(point, 2.0) - 1.0);
    gl_FragColor = texture2D(Texture, sampleUv) * color;
}
