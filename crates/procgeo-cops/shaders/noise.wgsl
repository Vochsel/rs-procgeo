struct Params {
    frequency: f32,
    amplitude: f32,
    lacunarity: f32,
    gain: f32,
    offset: vec2f,
    seed: u32,
    octaves: u32,
    noise_type: u32,
    _pad: vec3u,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> params: Params;

// ---- Hash functions ----

fn hash1(p: vec2f) -> f32 {
    var q = p;
    q = fract(q * vec2f(127.1, 311.7));
    q += dot(q, q + 19.19);
    return fract(q.x * q.y);
}

fn hash2(p: vec2f) -> vec2f {
    var q = p;
    q = vec2f(dot(q, vec2f(127.1, 311.7)), dot(q, vec2f(269.5, 183.3)));
    return fract(sin(q) * 43758.5453123);
}

// ---- Value noise with smoothstep interpolation (Perlin-style) ----

fn perlin(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash1(i + vec2f(0.0, 0.0));
    let b = hash1(i + vec2f(1.0, 0.0));
    let c = hash1(i + vec2f(0.0, 1.0));
    let d = hash1(i + vec2f(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// ---- Simplex-like noise ----

fn simplex(p: vec2f) -> f32 {
    let K1 = 0.366025404;
    let K2 = 0.211324865;

    let i = floor(p + (p.x + p.y) * K1);
    let a = p - i + (i.x + i.y) * K2;
    let m = step(a.yx, a.xy);
    let o = vec2f(m.x, 1.0 - m.x);
    let b = a - o + K2;
    let c = a - 1.0 + 2.0 * K2;

    let h = max(0.5 - vec3f(dot(a, a), dot(b, b), dot(c, c)), vec3f(0.0));
    let h2 = h * h;
    let h4 = h2 * h2;

    let n = vec3f(dot(a, hash2(i) - 0.5), dot(b, hash2(i + o) - 0.5), dot(c, hash2(i + 1.0) - 0.5));
    return dot(h4, n) * 70.0 * 0.5 + 0.5;
}

// ---- Worley (cellular) noise ----

fn worley(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);

    var min_dist = 8.0;
    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            let neighbor = vec2f(f32(dx), f32(dy));
            let cell = i + neighbor;
            let point = neighbor + hash2(cell);
            let dist = length(f - point);
            min_dist = min(min_dist, dist);
        }
    }
    return 1.0 - clamp(min_dist, 0.0, 1.0);
}

// ---- Fractal Brownian Motion ----

fn sample_noise(p: vec2f, noise_type: u32) -> f32 {
    if noise_type == 1u {
        return simplex(p);
    } else if noise_type == 2u {
        return worley(p);
    }
    return perlin(p);
}

fn fbm(p: vec2f, octaves: u32, lacunarity: f32, gain: f32, noise_type: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var total_amplitude = 0.0;
    for (var i: u32 = 0u; i < octaves; i++) {
        value += amplitude * sample_noise(p * frequency, noise_type);
        total_amplitude += amplitude;
        frequency *= lacunarity;
        amplitude *= gain;
    }
    return value / total_amplitude;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }

    let uv = vec2f(f32(gid.x), f32(gid.y)) / vec2f(f32(dims.x), f32(dims.y));
    let seed_offset = vec2f(f32(params.seed) * 0.1234, f32(params.seed) * 0.5678);
    let p = (uv + params.offset + seed_offset) * params.frequency;

    let n = fbm(p, params.octaves, params.lacunarity, params.gain, params.noise_type);
    let value = clamp(n * params.amplitude, 0.0, 1.0);

    textureStore(output, vec2i(gid.xy), vec4f(value, value, value, 1.0));
}
