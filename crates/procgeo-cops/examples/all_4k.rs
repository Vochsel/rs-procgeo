//! Renders all 5 COP demos at 4K (4096x4096) and prints timing for each.
//!
//! Run:  cargo run -p procgeo-cops --example all_4k --release
//!
//! Outputs to output_4k/ in the repo root.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use procgeo_cops::composite::{CompOp, CompositeCop, CompositeParams};
use procgeo_cops::custom::{CustomShaderCop, CustomShaderParams, ShaderLang};
use procgeo_cops::filter::{BlurCop, BlurParams, BlurType, SwirlCop, SwirlParams};
use procgeo_cops::generator::{
    CheckerboardCop, CheckerboardParams, NoiseCop, NoiseParams, NoiseType, RampCop, RampParams,
    RampType,
};
use procgeo_cops::prelude::*;

const SIZE: u32 = 4096;

fn save(img: &Image, name: &str) {
    save_image(
        img,
        &SaveImageParams {
            path: format!("output_4k/{name}.png"),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        },
    )
    .expect("save failed");
}

fn terrain_heightmap(ctx: &Arc<GpuContext>) -> Image {
    let hills = generate_cop(
        ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Simplex,
            frequency: 2.0,
            octaves: 4,
            lacunarity: 2.0,
            gain: 0.5,
            amplitude: 1.0,
            seed: 0,
            width: SIZE,
            height: SIZE,
            ..Default::default()
        },
    )
    .unwrap();

    let ridges = generate_cop(
        ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 6.0,
            octaves: 6,
            lacunarity: 2.2,
            gain: 0.45,
            amplitude: 0.4,
            seed: 42,
            width: SIZE,
            height: SIZE,
            ..Default::default()
        },
    )
    .unwrap();

    let detail = generate_cop(
        ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Worley,
            frequency: 12.0,
            octaves: 2,
            amplitude: 0.15,
            seed: 99,
            width: SIZE,
            height: SIZE,
            ..Default::default()
        },
    )
    .unwrap();

    let combined = CompositeCop
        .execute(
            ctx,
            &[&hills, &ridges],
            &CompositeParams {
                operation: CompOp::Add,
                mix: 0.6,
            },
        )
        .unwrap();

    let terrain = CompositeCop
        .execute(
            ctx,
            &[&combined, &detail],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.4,
            },
        )
        .unwrap();

    BlurCop
        .execute(
            ctx,
            &[&terrain],
            &BlurParams {
                radius_x: 2.0,
                radius_y: 2.0,
                ..Default::default()
            },
        )
        .unwrap()
}

fn neon_grid(ctx: &Arc<GpuContext>) -> Image {
    let checker = generate_cop(
        ctx,
        &CheckerboardCop,
        &CheckerboardParams {
            color_a: [0.9, 0.1, 0.9, 1.0],
            color_b: [0.1, 0.9, 0.9, 1.0],
            frequency: [12.0, 12.0],
            width: SIZE,
            height: SIZE,
        },
    )
    .unwrap();

    let glow = generate_cop(
        ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Radial,
            stops: vec![
                (0.0, [1.0, 1.0, 1.0, 1.0]),
                (0.6, [0.4, 0.2, 0.6, 1.0]),
                (1.0, [0.02, 0.01, 0.05, 1.0]),
            ],
            width: SIZE,
            height: SIZE,
        },
    )
    .unwrap();

    let composited = CompositeCop
        .execute(
            ctx,
            &[&checker, &glow],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 1.0,
            },
        )
        .unwrap();

    SwirlCop
        .execute(
            ctx,
            &[&composited],
            &SwirlParams {
                center: [0.5, 0.5],
                angle: 120.0,
                radius: 0.6,
            },
        )
        .unwrap()
}

fn marble_texture(ctx: &Arc<GpuContext>) -> Image {
    let base = generate_cop(
        ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 3.0,
            octaves: 4,
            amplitude: 1.0,
            seed: 7,
            width: SIZE,
            height: SIZE,
            ..Default::default()
        },
    )
    .unwrap();

    let veins = generate_cop(
        ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Worley,
            frequency: 5.0,
            octaves: 3,
            lacunarity: 2.5,
            gain: 0.6,
            amplitude: 1.0,
            seed: 33,
            width: SIZE,
            height: SIZE,
            ..Default::default()
        },
    )
    .unwrap();

    let color_ramp = generate_cop(
        ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Diagonal,
            stops: vec![
                (0.0, [0.92, 0.88, 0.82, 1.0]),
                (0.35, [0.85, 0.78, 0.70, 1.0]),
                (0.65, [0.70, 0.62, 0.55, 1.0]),
                (1.0, [0.55, 0.48, 0.42, 1.0]),
            ],
            width: SIZE,
            height: SIZE,
        },
    )
    .unwrap();

    let tinted = CompositeCop
        .execute(
            ctx,
            &[&color_ramp, &base],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.7,
            },
        )
        .unwrap();

    let marble = CompositeCop
        .execute(
            ctx,
            &[&tinted, &veins],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.3,
            },
        )
        .unwrap();

    BlurCop
        .execute(
            ctx,
            &[&marble],
            &BlurParams {
                radius_x: 1.5,
                radius_y: 1.5,
                ..Default::default()
            },
        )
        .unwrap()
}

fn plasma_shader(ctx: &Arc<GpuContext>) -> Image {
    const PLASMA_WGSL: &str = r#"
@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;

fn psin(x: f32) -> f32 {
    let t = fract(x / 6.2831853) * 6.2831853;
    return sin(t);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let uv = vec2f(f32(gid.x), f32(gid.y)) / vec2f(f32(dims.x), f32(dims.y));
    let p = uv * 8.0;

    var v = 0.0;
    v += psin(p.x + psin(p.y * 1.3 + 1.7));
    v += psin(p.y * 0.9 + psin(p.x * 1.1 + 2.3));
    v += psin(length(p - vec2f(4.0, 4.0)) * 1.5);
    v += psin(length(p - vec2f(2.0, 6.0)) * 2.0 + 0.5);
    v = v * 0.25 + 0.5;

    let r = psin(v * 6.2831853) * 0.5 + 0.5;
    let g = psin(v * 6.2831853 + 2.094) * 0.5 + 0.5;
    let b = psin(v * 6.2831853 + 4.189) * 0.5 + 0.5;
    textureStore(output, vec2i(gid.xy), vec4f(r, g, b, 1.0));
}
"#;

    generate_cop(
        ctx,
        &CustomShaderCop,
        &CustomShaderParams {
            source: PLASMA_WGSL.to_string(),
            language: ShaderLang::Wgsl,
            uniforms: HashMap::new(),
            width: SIZE,
            height: SIZE,
        },
    )
    .unwrap()
}

fn glow_effect(ctx: &Arc<GpuContext>) -> Image {
    let checker = generate_cop(
        ctx,
        &CheckerboardCop,
        &CheckerboardParams {
            color_a: [0.0, 0.0, 0.0, 1.0],
            color_b: [1.0, 0.7, 0.2, 1.0],
            frequency: [6.0, 6.0],
            width: SIZE,
            height: SIZE,
        },
    )
    .unwrap();

    let noise = generate_cop(
        ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Simplex,
            frequency: 8.0,
            octaves: 3,
            amplitude: 1.0,
            seed: 5,
            width: SIZE,
            height: SIZE,
            ..Default::default()
        },
    )
    .unwrap();

    let source = CompositeCop
        .execute(
            ctx,
            &[&checker, &noise],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.4,
            },
        )
        .unwrap();

    let bloom_wide = BlurCop
        .execute(
            ctx,
            &[&source],
            &BlurParams {
                blur_type: BlurType::Gaussian,
                radius_x: 20.0,
                radius_y: 20.0,
            },
        )
        .unwrap();

    let bloom_tight = BlurCop
        .execute(
            ctx,
            &[&source],
            &BlurParams {
                blur_type: BlurType::Gaussian,
                radius_x: 8.0,
                radius_y: 8.0,
            },
        )
        .unwrap();

    let bloom = CompositeCop
        .execute(
            ctx,
            &[&bloom_wide, &bloom_tight],
            &CompositeParams {
                operation: CompOp::Add,
                mix: 0.5,
            },
        )
        .unwrap();

    let glowed = CompositeCop
        .execute(
            ctx,
            &[&source, &bloom],
            &CompositeParams {
                operation: CompOp::Add,
                mix: 0.6,
            },
        )
        .unwrap();

    let vignette = generate_cop(
        ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Radial,
            stops: vec![
                (0.0, [1.0, 1.0, 1.0, 1.0]),
                (0.5, [0.9, 0.9, 0.9, 1.0]),
                (1.0, [0.15, 0.1, 0.05, 1.0]),
            ],
            width: SIZE,
            height: SIZE,
        },
    )
    .unwrap();

    CompositeCop
        .execute(
            ctx,
            &[&glowed, &vignette],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 1.0,
            },
        )
        .unwrap()
}

fn main() {
    let ctx = Arc::new(GpuContext::new_blocking().expect("Failed to init GPU"));

    println!("Rendering all 5 COP demos at 4K ({SIZE}x{SIZE})...\n");

    let demos: Vec<(&str, fn(&Arc<GpuContext>) -> Image)> = vec![
        ("terrain_heightmap", terrain_heightmap),
        ("neon_grid", neon_grid),
        ("marble_texture", marble_texture),
        ("plasma_shader", plasma_shader),
        ("glow_effect", glow_effect),
    ];

    let total_start = Instant::now();

    for (name, func) in &demos {
        let t0 = Instant::now();
        let img = func(&ctx);
        let gpu_ms = t0.elapsed().as_millis();

        let t1 = Instant::now();
        save(&img, name);
        let save_ms = t1.elapsed().as_millis();

        println!(
            "  {name:<20} gpu: {gpu_ms:>5}ms  save: {save_ms:>5}ms  total: {:>5}ms",
            gpu_ms + save_ms
        );
    }

    let total = total_start.elapsed().as_millis();
    println!("\nAll done in {total}ms — files in output_4k/");
}
