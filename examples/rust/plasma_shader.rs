//! Custom WGSL plasma shader — demonstrates the CustomShaderCop
//! with a hand-written GPU compute shader that generates a colorful
//! plasma/interference pattern.
//!
//! Run:  cargo run -p procgeo-cops --example plasma_shader
//!
//! Outputs `plasma_shader.png`.

use std::collections::HashMap;
use std::sync::Arc;

use procgeo_cops::prelude::*;
use procgeo_cops::custom::{CustomShaderCop, CustomShaderParams, ShaderLang};

const PLASMA_WGSL: &str = r#"
@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;

// Simple pseudo-sin via polynomial approximation
fn psin(x: f32) -> f32 {
    let t = fract(x / 6.2831853) * 6.2831853;
    return sin(t);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }

    let uv = vec2f(f32(gid.x), f32(gid.y)) / vec2f(f32(dims.x), f32(dims.y));

    // Plasma: sum of overlapping sine waves at different scales/angles
    let p = uv * 8.0;

    var v = 0.0;
    v += psin(p.x + psin(p.y * 1.3 + 1.7));
    v += psin(p.y * 0.9 + psin(p.x * 1.1 + 2.3));
    v += psin(length(p - vec2f(4.0, 4.0)) * 1.5);
    v += psin(length(p - vec2f(2.0, 6.0)) * 2.0 + 0.5);
    v = v * 0.25 + 0.5; // normalize to 0..1

    // Map value to vibrant colors via three offset sine waves
    let r = psin(v * 6.2831853) * 0.5 + 0.5;
    let g = psin(v * 6.2831853 + 2.094) * 0.5 + 0.5; // +120 degrees
    let b = psin(v * 6.2831853 + 4.189) * 0.5 + 0.5; // +240 degrees

    textureStore(output, vec2i(gid.xy), vec4f(r, g, b, 1.0));
}
"#;

fn main() {
    let ctx = Arc::new(
        GpuContext::new_blocking().expect("Failed to init GPU"),
    );

    let size = 512;

    let img = generate_cop(
        &ctx,
        &CustomShaderCop,
        &CustomShaderParams {
            source: PLASMA_WGSL.to_string(),
            language: ShaderLang::Wgsl,
            uniforms: HashMap::new(),
            width: size,
            height: size,
        },
    )
    .expect("plasma shader failed");

    save_image(
        &img,
        &SaveImageParams {
            path: "plasma_shader.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        },
    )
    .expect("save failed");

    println!(
        "Wrote plasma_shader.png ({}x{}) — custom WGSL plasma interference pattern",
        size, size
    );
}
