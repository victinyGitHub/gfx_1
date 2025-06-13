struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

struct AngleUniform {
    angle : f32,
};

@group(0) @binding(0)
var<uniform> u : AngleUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let c = cos(u.angle);
    let s = sin(u.angle);
    let rotated = vec2<f32>(
        in.position.x * c - in.position.y * s,
        in.position.x * s + in.position.y * c,
    );
    out.clip_position = vec4<f32>(rotated, 0.0, 1.0);
    out.color = vec3<f32>(-rotated, 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(cos(in.color.x), sin(in.color.y), tan(in.color.z), 1.0);
}
