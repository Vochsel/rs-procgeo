//! Retro neon grid — checkerboard composited with a radial gradient
//! and swirl distortion.
//!
//! Run:  cargo run -p procgeo-cops --example neon_grid
//!
//! Outputs `neon_grid.png`.

use std::sync::Arc;

use procgeo_cops::prelude::*;
use procgeo_cops::generator::{
    CheckerboardCop, CheckerboardParams,
    NoiseCop, NoiseParams, NoiseType,
    RampCop, RampParams, RampType,
};
use procgeo_cops::filter::{BlurCop, BlurParams, SwirlCop, SwirlParams};
use procgeo_cops::composite::{CompositeCop, CompositeParams, CompOp};

fn main() {
    let ctx = Arc::new(
        GpuContext::new_blocking().expect("Failed to init GPU"),
    );

    let size = 1024;

    // Primary neon checkerboard: magenta + cyan tiles
    let checker = generate_cop(
        &ctx,
        &CheckerboardCop,
        &CheckerboardParams {
            color_a: [1.0, 0.05, 0.9, 1.0], // hot magenta
            color_b: [0.0, 0.95, 1.0, 1.0],  // electric cyan
            frequency: [24.0, 24.0],
            width: size,
            height: size,
        },
    )
    .expect("checkerboard failed");

    // Secondary fine grid — creates moiré/scanline interference
    let fine_grid = generate_cop(
        &ctx,
        &CheckerboardCop,
        &CheckerboardParams {
            color_a: [0.8, 0.8, 0.8, 1.0],
            color_b: [1.0, 1.0, 1.0, 1.0],
            frequency: [96.0, 96.0],
            width: size,
            height: size,
        },
    )
    .expect("fine grid failed");

    // Multiply fine grid into main checker for subtle scanline texture
    let checker = CompositeCop
        .execute(
            &ctx,
            &[&checker, &fine_grid],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 0.15,
            },
        )
        .expect("scanline composite failed");

    // Radial gradient: hot white center, broader glow, deep edge falloff
    let glow = generate_cop(
        &ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Radial,
            stops: vec![
                (0.0, [1.0, 1.0, 1.0, 1.0]),
                (0.15, [1.0, 0.95, 1.0, 1.0]),
                (0.4, [0.85, 0.7, 0.95, 1.0]),
                (0.65, [0.45, 0.15, 0.65, 1.0]),
                (0.85, [0.12, 0.04, 0.25, 1.0]),
                (1.0, [0.02, 0.01, 0.06, 1.0]),
            ],
            width: size,
            height: size,
        },
    )
    .expect("ramp failed");

    // Diagonal warm→cool gradient for color shift across the image
    let color_shift = generate_cop(
        &ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Diagonal,
            stops: vec![
                (0.0, [1.0, 0.4, 0.2, 1.0]),   // warm orange
                (0.5, [0.8, 0.2, 0.9, 1.0]),    // purple
                (1.0, [0.1, 0.5, 1.0, 1.0]),    // blue
            ],
            width: size,
            height: size,
        },
    )
    .expect("color shift ramp failed");

    // Subtle simplex noise for organic color variation
    let color_var = generate_cop(
        &ctx,
        &NoiseCop,
        &NoiseParams {
            noise_type: NoiseType::Simplex,
            frequency: 3.0,
            octaves: 2,
            amplitude: 0.6,
            seed: 11,
            width: size,
            height: size,
            ..Default::default()
        },
    )
    .expect("color variation noise failed");

    // Multiply checker with glow for vignette
    let vignetted = CompositeCop
        .execute(
            &ctx,
            &[&checker, &glow],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 1.0,
            },
        )
        .expect("vignette composite failed");

    // Overlay diagonal color shift
    let shifted = CompositeCop
        .execute(
            &ctx,
            &[&vignetted, &color_shift],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.2,
            },
        )
        .expect("color shift composite failed");

    // Screen the noise for organic variation
    let varied = CompositeCop
        .execute(
            &ctx,
            &[&shifted, &color_var],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.1,
            },
        )
        .expect("variation composite failed");

    // Primary swirl — main warp
    let swirled = SwirlCop
        .execute(
            &ctx,
            &[&varied],
            &SwirlParams {
                center: [0.48, 0.52],
                angle: 180.0,
                radius: 0.8,
            },
        )
        .expect("swirl 1 failed");

    // Secondary counter-swirl — off-center for asymmetric complexity
    let swirled2 = SwirlCop
        .execute(
            &ctx,
            &[&swirled],
            &SwirlParams {
                center: [0.65, 0.35],
                angle: -60.0,
                radius: 0.3,
            },
        )
        .expect("swirl 2 failed");

    // Wide bloom for soft neon haze
    let bloom_wide = BlurCop
        .execute(
            &ctx,
            &[&swirled2],
            &BlurParams {
                radius_x: 16.0,
                radius_y: 16.0,
                ..Default::default()
            },
        )
        .expect("wide bloom failed");

    // Tight bloom for sharp glow on tile edges
    let bloom_tight = BlurCop
        .execute(
            &ctx,
            &[&swirled2],
            &BlurParams {
                radius_x: 4.0,
                radius_y: 4.0,
                ..Default::default()
            },
        )
        .expect("tight bloom failed");

    // Layer: sharp image + tight glow + soft haze
    let with_tight = CompositeCop
        .execute(
            &ctx,
            &[&swirled2, &bloom_tight],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.35,
            },
        )
        .expect("tight bloom composite failed");

    let with_bloom = CompositeCop
        .execute(
            &ctx,
            &[&with_tight, &bloom_wide],
            &CompositeParams {
                operation: CompOp::Screen,
                mix: 0.3,
            },
        )
        .expect("wide bloom composite failed");

    save_image(
        &with_bloom,
        &SaveImageParams {
            path: "neon_grid.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        },
    )
    .expect("save failed");

    println!("Wrote neon_grid.png ({}x{}) — neon checkerboard with radial glow + swirl", size, size);
}
