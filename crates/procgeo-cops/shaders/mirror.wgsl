struct Params {
    axis: u32,     // 0=X, 1=Y
    offset: f32,   // 0.0-1.0 normalized mirror line position
    _pad: vec2u,
}

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    var src = vec2i(gid.xy);
    if params.axis == 0u {
        let mirror_x = params.offset * f32(dims.x);
        if f32(gid.x) > mirror_x {
            src.x = clamp(i32(2.0 * mirror_x) - src.x, 0, i32(dims.x) - 1);
        }
    } else {
        let mirror_y = params.offset * f32(dims.y);
        if f32(gid.y) > mirror_y {
            src.y = clamp(i32(2.0 * mirror_y) - src.y, 0, i32(dims.y) - 1);
        }
    }
    textureStore(output, vec2i(gid.xy), textureLoad(input, src, 0));
}
