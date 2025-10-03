#version 140

out vec4 out_color;

uniform vec4 resolution;
uniform float time;
uniform float float_test1,float_test2;
uniform int int_test1;
uniform bool bool_test1;

void main()
{
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    vec3 c = vec3(float_test1,float_test2,float(int_test1 == 8));
    out_color = vec4(c * float(bool_test1),1.0);
}
