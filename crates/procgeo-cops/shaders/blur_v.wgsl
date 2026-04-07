struct Params {
    radius: i32,
    is_gaussian: u32,
    _pad: vec2u,
}

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: Params;

fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let sigma = max(f32(params.radius) / 3.0, 0.001);
    var color = vec4f(0.0);
    var weight_sum = 0.0;
    for (var dy = -params.radius; dy <= params.radius; dy++) {
        let sy = clamp(i32(gid.y) + dy, 0, i32(dims.y) - 1);
        let w = select(1.0, gaussian_weight(f32(dy), sigma), params.is_gaussian != 0u);
        color += textureLoad(input, vec2i(i32(gid.x), sy), 0) * w;
        weight_sum += w;
    }
    textureStore(output, vec2i(gid.xy), color / weight_sum);
}
