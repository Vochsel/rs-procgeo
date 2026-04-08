//! Procedural terrain heightmap using layered fBm noise.
//!
//! Run:  cargo run -p procgeo-cops --example terrain_heightmap
//!
//! Outputs `terrain_heightmap.png` — a grayscale heightmap suitable
//! for displacement mapping or terrain generation.

use std::sync::Arc;

use procgeo_cops::prelude::*;
use procgeo_cops::generator::{NoiseCop, NoiseParams, NoiseType, ConstantCop, ConstantParams};
use procgeo_cops::filter::{BlurCop, BlurParams};
use procgeo_cops::composite::{CompositeCop, CompositeParams, CompOp};

fn main() {
    let ctx = Arc::new(
        GpuContext::new_blocking().expect("Failed to init GPU — ensure a GPU/driver is available"),
    );

    let size = 1024;

    // Layer 1: broad continental shapes (low-frequency Simplex fBm)
    let hills = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Simplex,
            frequency: 1.5,
            octaves: 6,
            lacunarity: 2.0,
            gain: 0.5,
            amplitude: 1.0,
            seed: 0,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("hills noise failed");

    // Layer 2: sharp mountain ridges (Perlin, higher frequency, more octaves)
    let ridges = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 4.0,
            octaves: 8,
            lacunarity: 2.1,
            gain: 0.55,
            amplitude: 0.7,
            seed: 42,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("ridges noise failed");

    // Layer 3: fine rocky detail & erosion channels (Worley cellular)
    let detail = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Worley,
            frequency: 18.0,
            octaves: 3,
            lacunarity: 2.3,
            gain: 0.5,
            amplitude: 0.35,
            seed: 99,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("detail noise failed");

    // Layer 4: ultra-fine surface grain
    let grain = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 40.0,
            octaves: 2,
            amplitude: 0.12,
            seed: 7,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("grain noise failed");

    // Gentle power-curve on hills to separate peaks from plains
    let hills_contrast = CompositeCop
        .execute(
            &ctx,
            &[&hills, &hills],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.4,
            },
        )
        .expect("hills contrast failed");

    // Screen ridges aggressively — they form the bright mountain peaks
    let with_ridges = CompositeCop
        .execute(
            &ctx,
            &[&hills_contrast, &ridges],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 1.0,
            },
        )
        .expect("ridges composite failed");

    // Subtract Worley lightly for erosion channels without eating too much brightness
    let with_valleys = CompositeCop
        .execute(
            &ctx,
            &[&with_ridges, &detail],
            &CompositeParams {
                operation: CompOp::Subtract,
                mix: 0.3,
            },
        )
        .expect("valley carving failed");

    // Multiply with grain for micro-detail texture
    let terrain = CompositeCop
        .execute(
            &ctx,
            &[&with_valleys, &grain],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.15,
            },
        )
        .expect("grain composite failed");

    // Light self-multiply for contrast — don't over-darken
    let contrasted = CompositeCop
        .execute(
            &ctx,
            &[&terrain, &terrain],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.3,
            },
        )
        .expect("contrast boost failed");

    // Screen back the original hills at low mix to recover some brightness
    let brightened = CompositeCop
        .execute(
            &ctx,
            &[&contrasted, &hills],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.25,
            },
        )
        .expect("brightness recovery failed");

    // Very light blur — preserve detail
    let smoothed = BlurCop
        .execute(
            &ctx,
            &[&brightened],
            &BlurParams {
                radius_x: 0.5,
                radius_y: 0.5,
                ..Default::default()
            },
        )
        .expect("blur failed");

    save_image(
        &smoothed,
        &SaveImageParams {
            path: "terrain_heightmap.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Sixteen,
        },
    )
    .expect("save failed");

    println!(
        "Wrote terrain_heightmap.png ({}x{}) — 3-layer fBm terrain heightmap",
        size, size
    );
}
