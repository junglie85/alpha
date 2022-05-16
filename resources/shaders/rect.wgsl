// Vertex shader

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
};

struct Model {
    matrix: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> model: Model;

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] color: vec3<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.color = color;
    out.clip_position = model.matrix * vec4<f32>(position, 1.0);
    return out;
}

// Fragment shader

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
