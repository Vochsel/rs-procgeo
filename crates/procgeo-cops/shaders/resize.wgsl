struct Params {
    src_width: u32,
    src_height: u32,
    filter_mode: u32, // 0=Nearest, 1=Bilinear
    _pad: u32,
}

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: Params;

fn sample_bilinear(tex: texture_2d<f32>, pos: vec2f, src_dims: vec2u) -> vec4f {
    let px = pos - 0.5;
    let x0 = i32(floor(px.x));
    let y0 = i32(floor(px.y));
    let fx = fract(px.x);
    let fy = fract(px.y);
    let c00 = textureLoad(tex, vec2i(clamp(x0,     0, i32(src_dims.x) - 1), clamp(y0,     0, i32(src_dims.y) - 1)), 0);
    let c10 = textureLoad(tex, vec2i(clamp(x0 + 1, 0, i32(src_dims.x) - 1), clamp(y0,     0, i32(src_dims.y) - 1)), 0);
    let c01 = textureLoad(tex, vec2i(clamp(x0,     0, i32(src_dims.x) - 1), clamp(y0 + 1, 0, i32(src_dims.y) - 1)), 0);
    let c11 = textureLoad(tex, vec2i(clamp(x0 + 1, 0, i32(src_dims.x) - 1), clamp(y0 + 1, 0, i32(src_dims.y) - 1)), 0);
    return mix(mix(c00, c10, fx), mix(c01, c11, fx), fy);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dst_dims = textureDimensions(output);
    if gid.x >= dst_dims.x || gid.y >= dst_dims.y { return; }

    let src_dims = vec2u(params.src_width, params.src_height);

    // Map output pixel to source coordinate
    let src_pos = vec2f(
        (f32(gid.x) + 0.5) / f32(dst_dims.x) * f32(src_dims.x),
        (f32(gid.y) + 0.5) / f32(dst_dims.y) * f32(src_dims.y)
    );

    var color: vec4f;
    if params.filter_mode == 1u {
        color = sample_bilinear(input, src_pos, src_dims);
    } else {
        let sp = vec2i(
            clamp(i32(src_pos.x), 0, i32(src_dims.x) - 1),
            clamp(i32(src_pos.y), 0, i32(src_dims.y) - 1)
        );
        color = textureLoad(input, sp, 0);
    }
    textureStore(output, vec2i(gid.xy), color);
}
