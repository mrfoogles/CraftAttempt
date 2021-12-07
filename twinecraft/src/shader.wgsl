// Vertex shader

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] color: vec3<f32>;
    [[location(2)]] worldpos: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
    [[location(1)]] worldpos: vec3<f32>;
};

[[block]]
struct VertUniforms {
    time: f32;
};
[[group(0), binding(0)]]
var<uniform> vu: VertUniforms;

[[block]]
struct FragUniforms {
    time: f32;
};
[[group(0), binding(1)]]
var<uniform> fu: FragUniforms;

[[block]]
struct Camera {
    view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: Camera;

[[stage(vertex)]]
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    var pos: vec3<f32> = model.position + model.worldpos;
    pos = pos * 1.;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.worldpos = model.worldpos;
    return out;
}

// Fragment shader

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.color,1.);
}