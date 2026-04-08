//! Procedural marble texture — Worley veins composited with Perlin base
//! and a warm color ramp.
//!
//! Run:  cargo run -p procgeo-cops --example marble_texture
//!
//! Outputs `marble_texture.png`.

use std::sync::Arc;

use procgeo_cops::composite::{CompOp, CompositeCop, CompositeParams};
use procgeo_cops::filter::{BlurCop, BlurParams};
use procgeo_cops::generator::{NoiseCop, NoiseParams, NoiseType, RampCop, RampParams, RampType};
use procgeo_cops::prelude::*;

fn main() {
    let ctx = Arc::new(GpuContext::new_blocking().expect("Failed to init GPU"));

    let size = 1024;

    // Base layer: smooth Perlin stone texture
    let base = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 2.5,
            octaves: 5,
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

    // Primary vein network: Worley cellular (dark veins in marble)
    let veins = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Worley,
            frequency: 4.0,
            octaves: 4,
            lacunarity: 2.2,
            gain: 0.6,
            amplitude: 1.0,
            seed: 33,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("veins noise failed");

    // Secondary fine veins: higher frequency for capillary-like detail
    let fine_veins = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Worley,
            frequency: 10.0,
            octaves: 3,
            lacunarity: 2.5,
            gain: 0.55,
            amplitude: 0.8,
            seed: 77,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("fine veins noise failed");

    // Subtle grain: high-freq Perlin for stone crystal texture
    let grain = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 30.0,
            octaves: 2,
            amplitude: 0.3,
            seed: 55,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("grain noise failed");

    // Color ramp: warm marble tones with more range
    let color_ramp = generate_cop(
        &ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Diagonal,
            stops: vec![
                (0.0, [0.95, 0.92, 0.88, 1.0]),  // bright cream
                (0.25, [0.90, 0.85, 0.78, 1.0]), // warm ivory
                (0.5, [0.78, 0.70, 0.62, 1.0]),  // medium tan
                (0.75, [0.62, 0.55, 0.48, 1.0]), // medium stone
                (1.0, [0.48, 0.40, 0.35, 1.0]),  // dark stone
            ],
            width: size,
            height: size,
        },
    )
    .expect("color ramp failed");

    // Build base: bright color ramp with gentle Perlin modulation
    let tinted = CompositeCop
        .execute(
            &ctx,
            &[&color_ramp, &base],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.4,
            },
        )
        .expect("tint failed");

    // Screen the base brighter — marble is predominantly light
    let bright_base = CompositeCop
        .execute(
            &ctx,
            &[&tinted, &color_ramp],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.5,
            },
        )
        .expect("brighten failed");

    // Create sharp vein edges by taking the difference between two Worley scales
    // This highlights the boundaries where cell regions change
    let vein_edges = CompositeCop
        .execute(
            &ctx,
            &[&veins, &fine_veins],
            &CompositeParams {
                operation: CompOp::Difference,
                mix: 1.0,
            },
        )
        .expect("vein edge difference failed");

    // Power-sharpen the edges: self-multiply to darken darks
    let edges_sharp = CompositeCop
        .execute(
            &ctx,
            &[&vein_edges, &vein_edges],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 1.0,
            },
        )
        .expect("edge sharpen failed");

    // Also use the primary veins as broader dark regions
    let veins_sharp = CompositeCop
        .execute(
            &ctx,
            &[&veins, &veins],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 1.0,
            },
        )
        .expect("vein sharpen failed");

    // Multiply broad veins into the bright base
    let with_veins = CompositeCop
        .execute(
            &ctx,
            &[&bright_base, &veins_sharp],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.45,
            },
        )
        .expect("vein multiply failed");

    // Subtract sharp edges for crisp vein lines
    let with_fine = CompositeCop
        .execute(
            &ctx,
            &[&with_veins, &edges_sharp],
            &CompositeParams {
                operation: CompOp::Subtract,
                mix: 0.35,
            },
        )
        .expect("edge subtract failed");

    // Multiply grain lightly for crystal texture
    let with_grain = CompositeCop
        .execute(
            &ctx,
            &[&with_fine, &grain],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.08,
            },
        )
        .expect("grain composite failed");

    // Screen back the ramp strongly to lift brightness in non-vein areas
    let recovered = CompositeCop
        .execute(
            &ctx,
            &[&with_grain, &color_ramp],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.55,
            },
        )
        .expect("brightness recovery failed");

    // Minimal blur for polished stone look
    let smoothed = BlurCop
        .execute(
            &ctx,
            &[&recovered],
            &BlurParams {
                radius_x: 0.3,
                radius_y: 0.3,
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
