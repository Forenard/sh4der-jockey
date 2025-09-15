#version 440

out vec4 out_color;

uniform vec4 resolution;
uniform float time;
uniform sampler2D spout_texture;

void main()
{
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    out_color = vec4(mix(texture(spout_texture,uv).rgb,vec3(0,0,1),step(fract(time),0.5)),1);
}