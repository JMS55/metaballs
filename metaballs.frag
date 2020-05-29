#version 450

layout(set = 0, binding = 0) uniform TimeUniform {
    float time;
};
layout(set = 0, binding = 1) uniform ScreenSizeUniform {
    vec2 screen_size;
};

layout(location = 1) in vec2 texture_coordinate;
layout(location = 0) out vec4 color;

float metaball(vec2 point, vec2 center) {
    return 1.0 / (pow(point.x - center.x, 2.0) + pow(point.y - center.y, 2.0));
}

vec3 normal(vec2 point, vec2 center) {
    vec2 xy = point - center;
    float z = sqrt(dot(xy, xy) - 1.0);
    return normalize(vec3(xy, z));
}

void main() {
    vec2 uv = (texture_coordinate * 2.0) - 1.0;
    float aspectRatio = screen_size.x / screen_size.y;
    uv.y /= aspectRatio;

    float t = time * 1.3;
    vec2 c1 = vec2(0.0, 0.0);
    vec2 c2 = vec2(cos(t) / 1.5, sin(t) / 4.0);
    vec2 c3 = vec2(-c2.y, c2.x);
    vec2 c4 = vec2(cos(t), sin(t)) / 1.5;

    float m1 = metaball(uv, c1);
    float m2 = metaball(uv, c2);
    float m3 = metaball(uv, c3);
    float m4 = metaball(uv, c4);
    float m = m1 + m2 + m3 + m4;

    vec3 n1 = normal(uv, c1) * m1;
    vec3 n2 = normal(uv, c2) * m2;
    vec3 n3 = normal(uv, c3) * m3;
    vec3 n4 = normal(uv, c4) * m4;
    vec3 n = (n1 + n2 + n3 + n4) / m;

    color = vec4(0.2, 0.2, 0.2, 1.0);
    if (m >= 70.0) {
        color.rgb = (n * 5.0).gbr;
    }
}
