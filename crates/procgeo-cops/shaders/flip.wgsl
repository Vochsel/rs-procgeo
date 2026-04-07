struct Params {
    horizontal: u32,
    vertical: u32,
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
    if params.horizontal != 0u { src.x = i32(dims.x) - 1 - src.x; }
    if params.vertical != 0u { src.y = i32(dims.y) - 1 - src.y; }
    textureStore(output, vec2i(gid.xy), textureLoad(input, src, 0));
}
