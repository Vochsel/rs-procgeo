struct Params {
    r_src: u32,
    g_src: u32,
    b_src: u32,
    a_src: u32,
}

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: Params;

fn pick(color: vec4f, src: u32) -> f32 {
    switch src {
        case 0u: { return color.r; }
        case 1u: { return color.g; }
        case 2u: { return color.b; }
        case 3u: { return color.a; }
        case 4u: { return 1.0; }
        default: { return 0.0; }
    }
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let c = textureLoad(input, vec2i(gid.xy), 0);
    textureStore(output, vec2i(gid.xy), vec4f(
        pick(c, params.r_src),
        pick(c, params.g_src),
        pick(c, params.b_src),
        pick(c, params.a_src)
    ));
}
