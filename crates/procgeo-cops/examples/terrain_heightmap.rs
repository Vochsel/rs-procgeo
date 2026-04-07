//! Procedural terrain heightmap using layered fBm noise.
//!
//! Run:  cargo run -p procgeo-cops --example terrain_heightmap
//!
//! Outputs `terrain_heightmap.png` — a grayscale heightmap suitable
//! for displacement mapping or terrain generation.

use std::sync::Arc;

use procgeo_cops::prelude::*;
use procgeo_cops::generator::{NoiseCop, NoiseParams, NoiseType};
use procgeo_cops::filter::{BlurCop, BlurParams};
use procgeo_cops::composite::{CompositeCop, CompositeParams, CompOp};

fn main() {
    let ctx = Arc::new(
        GpuContext::new_blocking().expect("Failed to init GPU — ensure a GPU/driver is available"),
    );

    let size = 512;

    // Layer 1: broad rolling hills (low-frequency Simplex fBm)
    let hills = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Simplex,
            frequency: 2.0,
            octaves: 4,
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

    // Layer 2: medium ridges (Perlin, higher frequency)
    let ridges = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Perlin,
            frequency: 6.0,
            octaves: 6,
            lacunarity: 2.2,
            gain: 0.45,
            amplitude: 0.4,
            seed: 42,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("ridges noise failed");

    // Layer 3: fine rocky detail (Worley cellular)
    let detail = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Worley,
            frequency: 12.0,
            octaves: 2,
            amplitude: 0.15,
            seed: 99,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("detail noise failed");

    // Composite: hills + ridges (additive blend)
    let combined = CompositeCop
        .execute(
            &ctx,
            &[&hills, &ridges],
            &CompositeParams {
                operation: CompOp::Add,
                mix: 0.6,
            },
        )
        .expect("composite hills+ridges failed");

    // Add fine detail via screen blend
    let terrain = CompositeCop
        .execute(
            &ctx,
            &[&combined, &detail],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.4,
            },
        )
        .expect("composite detail failed");

    // Gentle blur to smooth sharp cellular edges
    let smoothed = BlurCop
        .execute(
            &ctx,
            &[&terrain],
            &BlurParams {
                radius_x: 2.0,
                radius_y: 2.0,
                ..Default::default()
            },
        )
        .expect("blur failed");

    save_image(
        &smoothed,
        &SaveImageParams {
            path: "terrain_heightmap.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        },
    )
    .expect("save failed");

    println!(
        "Wrote terrain_heightmap.png ({}x{}) — 3-layer fBm terrain heightmap",
        size, size
    );
}
