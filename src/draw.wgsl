[[group(0), binding(0)]]
var r_texture: texture_2d<f32>;
[[group(0), binding(1)]]
var r_sampler: sampler;

struct VertOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

const positions : array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    // Upper left triangle
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, 1.0),

    // Lower right triangle
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(1.0, 1.0)
);

const uv : array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    // Upper left triangle
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),

    // Lower right triangle
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0)
);

[[stage(vertex)]]
fn vs_texture(
    [[builtin(vertex_index)]] vertex_index: u32
) -> VertOutput {
    // Should be unnessary when naga closes this: https://github.com/gfx-rs/naga/issues/346
    var temp_pos: array<vec2<f32>, 6> = positions;
    var temp_uv: array<vec2<f32>, 6> = uv;

    var out: VertOutput;
    out.position = vec4<f32>(temp_pos[vertex_index].x, temp_pos[vertex_index].y, 0.0, 1.0);
    out.uv = temp_uv[vertex_index];
    return out;
}

[[stage(fragment)]]
fn fs_texture(in: VertOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(r_texture, r_sampler, in.uv);
}