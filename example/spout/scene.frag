#version 440

uniform float time;
uniform vec4 resolution;

out vec4 out_color;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    vec3 col = 0.5 + 0.5 * cos(time + uv.xyx + vec3(0.0, 2.0, 4.0));
    out_color = vec4(col, 1.0);
}
