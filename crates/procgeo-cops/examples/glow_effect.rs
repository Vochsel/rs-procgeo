//! Bloom / glow post-process — generates a sharp pattern, blurs a copy,
//! and additively composites the glow back on top.
//!
//! Run:  cargo run -p procgeo-cops --example glow_effect
//!
//! Outputs `glow_effect.png`.

use std::sync::Arc;

use procgeo_cops::composite::{CompOp, CompositeCop, CompositeParams};
use procgeo_cops::filter::{BlurCop, BlurParams, BlurType};
use procgeo_cops::generator::{
    CheckerboardCop, CheckerboardParams, NoiseCop, NoiseParams, NoiseType, RampCop, RampParams,
    RampType,
};
use procgeo_cops::prelude::*;

fn main() {
    let ctx = Arc::new(GpuContext::new_blocking().expect("Failed to init GPU"));

    let size = 512;

    // Base image: bright checkerboard "emissive" pattern
    let checker = generate_cop(
        &ctx,
        &CheckerboardCop,
        &CheckerboardParams {
            color_a: [0.0, 0.0, 0.0, 1.0],
            color_b: [1.0, 0.7, 0.2, 1.0], // warm orange
            frequency: [6.0, 6.0],
            width: size,
            height: size,
        },
    )
    .expect("checkerboard failed");

    // Add some noise variation so it's not perfectly uniform
    let noise = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Simplex,
            frequency: 8.0,
            octaves: 3,
            amplitude: 1.0,
            seed: 5,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("noise failed");

    // Multiply noise into the checker for variation
    let source = CompositeCop
        .execute(
            &ctx,
            &[&checker, &noise],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.4,
            },
        )
        .expect("source composite failed");

    // Bloom pass 1: heavy blur of the source
    let bloom_wide = BlurCop
        .execute(
            &ctx,
            &[&source],
            &BlurParams {
                blur_type: BlurType::Gaussian,
                radius_x: 20.0,
                radius_y: 20.0,
            },
        )
        .expect("wide blur failed");

    // Bloom pass 2: medium blur for tighter glow
    let bloom_tight = BlurCop
        .execute(
            &ctx,
            &[&source],
            &BlurParams {
                blur_type: BlurType::Gaussian,
                radius_x: 8.0,
                radius_y: 8.0,
            },
        )
        .expect("tight blur failed");

    // Combine both bloom layers (additive)
    let bloom = CompositeCop
        .execute(
            &ctx,
            &[&bloom_wide, &bloom_tight],
            &CompositeParams {
                operation: CompOp::Add,
                mix: 0.5,
            },
        )
        .expect("bloom combine failed");

    // Add bloom back onto the original source
    let glowed = CompositeCop
        .execute(
            &ctx,
            &[&source, &bloom],
            &CompositeParams {
                operation: CompOp::Add,
                mix: 0.6,
            },
        )
        .expect("glow composite failed");

    // Vignette: darken edges with radial ramp
    let vignette = generate_cop(
        &ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Radial,
            stops: vec![
                (0.0, [1.0, 1.0, 1.0, 1.0]),
                (0.5, [0.9, 0.9, 0.9, 1.0]),
                (1.0, [0.15, 0.1, 0.05, 1.0]),
            ],
            width: size,
            height: size,
        },
    )
    .expect("vignette ramp failed");

    let final_img = CompositeCop
        .execute(
            &ctx,
            &[&glowed, &vignette],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 1.0,
            },
        )
        .expect("vignette composite failed");

    save_image(
        &final_img,
        &SaveImageParams {
            path: "glow_effect.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        },
    )
    .expect("save failed");

    println!(
        "Wrote glow_effect.png ({}x{}) — dual-layer bloom with vignette",
        size, size
    );
}
