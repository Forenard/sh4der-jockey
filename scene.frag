#version 140

out vec4 out_color;

uniform vec4 resolution;
uniform float time;
uniform float brightness;
uniform float multi1,multi2,multi3;

void main()
{
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    float b = brightness;
    vec3 c = vec3(multi1,multi2,multi3);
    out_color = vec4(b * c + vec3(0.25 * sin(time)),1.0);
}
