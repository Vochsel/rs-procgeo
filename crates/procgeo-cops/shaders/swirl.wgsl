struct Params {
    center: vec2f,
    angle: f32,
    radius: f32,
}

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let uv = (vec2f(f32(gid.x), f32(gid.y)) + 0.5) / vec2f(f32(dims.x), f32(dims.y));
    let delta = uv - params.center;
    let dist = length(delta);
    var src_uv = uv;
    if dist < params.radius && params.radius > 0.0 {
        let factor = 1.0 - (dist / params.radius);
        let twist = params.angle * 3.14159265 / 180.0 * factor * factor;
        src_uv = params.center + vec2f(
            delta.x * cos(twist) - delta.y * sin(twist),
            delta.x * sin(twist) + delta.y * cos(twist)
        );
    }
    let sp = vec2i(
        clamp(i32(src_uv.x * f32(dims.x)), 0, i32(dims.x) - 1),
        clamp(i32(src_uv.y * f32(dims.y)), 0, i32(dims.y) - 1)
    );
    textureStore(output, vec2i(gid.xy), textureLoad(input, sp, 0));
}
