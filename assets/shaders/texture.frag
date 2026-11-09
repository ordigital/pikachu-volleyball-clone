#version 100
precision mediump float;
varying mediump vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() { gl_FragColor = texture2D(Texture, uv) * color; }
