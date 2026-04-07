struct Params {
    color_a: vec4f,
    color_b: vec4f,
    frequency: vec2f,
    _pad: vec2f,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let uv = vec2f(f32(gid.x), f32(gid.y)) / vec2f(f32(dims.x), f32(dims.y));
    let cell = vec2i(floor(uv * params.frequency));
    let checker = (cell.x + cell.y) % 2;
    let color = select(params.color_a, params.color_b, checker == 1);
    textureStore(output, vec2i(gid.xy), color);
}
