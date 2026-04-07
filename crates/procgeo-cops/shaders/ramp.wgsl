struct Params {
    ramp_type: u32,
    stop_count: u32,
    _pad: vec2u,
}

struct Stop {
    position: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    color: vec4f,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<storage, read> stops: array<Stop>;

fn sample_ramp(t: f32) -> vec4f {
    let count = params.stop_count;
    if count == 0u {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    if count == 1u {
        return stops[0].color;
    }

    // Clamp t to [0, 1]
    let tc = clamp(t, 0.0, 1.0);

    // Find the two surrounding stops
    if tc <= stops[0].position {
        return stops[0].color;
    }
    if tc >= stops[count - 1u].position {
        return stops[count - 1u].color;
    }

    for (var i: u32 = 0u; i < count - 1u; i++) {
        let a = stops[i];
        let b = stops[i + 1u];
        if tc >= a.position && tc <= b.position {
            let range = b.position - a.position;
            var blend = 0.0;
            if range > 0.0001 {
                blend = (tc - a.position) / range;
            }
            return mix(a.color, b.color, blend);
        }
    }

    return stops[count - 1u].color;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }

    let uv = vec2f(f32(gid.x), f32(gid.y)) / vec2f(f32(dims.x), f32(dims.y));

    var t: f32;
    switch params.ramp_type {
        case 0u: { // Linear
            t = uv.x;
        }
        case 1u: { // Radial
            let centered = uv - vec2f(0.5);
            t = length(centered) * 2.0;
        }
        case 2u: { // Box
            let centered = abs(uv - vec2f(0.5)) * 2.0;
            t = max(centered.x, centered.y);
        }
        case 3u: { // Diagonal
            t = (uv.x + uv.y) * 0.5;
        }
        default: {
            t = uv.x;
        }
    }

    let color = sample_ramp(t);
    textureStore(output, vec2i(gid.xy), color);
}
