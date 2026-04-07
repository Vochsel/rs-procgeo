//! Procedural marble texture — Worley veins composited with Perlin base
//! and a warm color ramp.
//!
//! Run:  cargo run -p procgeo-cops --example marble_texture
//!
//! Outputs `marble_texture.png`.

use std::sync::Arc;

use procgeo_cops::prelude::*;
use procgeo_cops::generator::{
    NoiseCop, NoiseParams, NoiseType,
    RampCop, RampParams, RampType,
};
use procgeo_cops::filter::{BlurCop, BlurParams};
use procgeo_cops::composite::{CompositeCop, CompositeParams, CompOp};

fn main() {
    let ctx = Arc::new(
        GpuContext::new_blocking().expect("Failed to init GPU"),
    );

    let size = 512;

    // Base layer: smooth Perlin noise (warm stone base)
    let base = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 3.0,
            octaves: 4,
            lacunarity: 2.0,
            gain: 0.5,
            amplitude: 1.0,
            seed: 7,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("base noise failed");

    // Vein layer: Worley cellular noise (sharp vein-like cracks)
    let veins = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Worley,
            frequency: 5.0,
            octaves: 3,
            lacunarity: 2.5,
            gain: 0.6,
            amplitude: 1.0,
            seed: 33,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("veins noise failed");

    // Color ramp: warm marble tones (diagonal gives directional grain)
    let color_ramp = generate_cop(
        &ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Diagonal,
            stops: vec![
                (0.0, [0.92, 0.88, 0.82, 1.0]), // warm cream
                (0.35, [0.85, 0.78, 0.70, 1.0]), // light tan
                (0.65, [0.70, 0.62, 0.55, 1.0]), // medium stone
                (1.0, [0.55, 0.48, 0.42, 1.0]),  // dark stone
            ],
            width: size,
            height: size,
        },
    )
    .expect("color ramp failed");

    // Combine base with color ramp via multiply
    let tinted = CompositeCop
        .execute(
            &ctx,
            &[&color_ramp, &base],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.7,
            },
        )
        .expect("tint failed");

    // Overlay veins using screen blend (bright veins)
    let marble = CompositeCop
        .execute(
            &ctx,
            &[&tinted, &veins],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.3,
            },
        )
        .expect("vein composite failed");

    // Soft blur for natural look
    let smoothed = BlurCop
        .execute(
            &ctx,
            &[&marble],
            &BlurParams {
                radius_x: 1.5,
                radius_y: 1.5,
                ..Default::default()
            },
        )
        .expect("blur failed");

    save_image(
        &smoothed,
        &SaveImageParams {
            path: "marble_texture.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        },
    )
    .expect("save failed");

    println!(
        "Wrote marble_texture.png ({}x{}) — procedural marble with Worley veins",
        size, size
    );
}
