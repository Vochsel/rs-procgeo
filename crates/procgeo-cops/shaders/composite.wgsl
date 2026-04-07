struct Params {
    operation: u32, // 0=Over, 1=Add, 2=Multiply, 3=Screen, 4=Subtract, 5=Difference, 6=Min, 7=Max
    mix_factor: f32,
    _pad: vec2u,
}

@group(0) @binding(0) var input_a: texture_2d<f32>;
@group(0) @binding(1) var input_b: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let a = textureLoad(input_a, vec2i(gid.xy), 0);
    let b = textureLoad(input_b, vec2i(gid.xy), 0);
    var result: vec4f;
    switch params.operation {
        case 0u: { result = vec4f(a.rgb * a.a + b.rgb * (1.0 - a.a), a.a + b.a * (1.0 - a.a)); }
        case 1u: { result = a + b; }
        case 2u: { result = a * b; }
        case 3u: { result = vec4f(1.0) - (vec4f(1.0) - a) * (vec4f(1.0) - b); }
        case 4u: { result = a - b; }
        case 5u: { result = abs(a - b); }
        case 6u: { result = min(a, b); }
        case 7u: { result = max(a, b); }
        default: { result = a; }
    }
    result = mix(a, result, params.mix_factor);
    textureStore(output, vec2i(gid.xy), result);
}
