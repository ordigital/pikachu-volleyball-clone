#version 100
precision mediump float;
varying mediump vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    // Bufor zawiera już obraz złożony na nieprzezroczystym tle.
    // Nie wolno po raz drugi mnożyć RGB przez alfa z tego bufora.
    gl_FragColor = vec4(texture2D(Texture, uv).rgb * color.rgb, 1.0);
}
