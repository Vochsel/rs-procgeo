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
    RampCop, RampParams, RampType,
};
use procgeo_cops::filter::{SwirlCop, SwirlParams};
use procgeo_cops::composite::{CompositeCop, CompositeParams, CompOp};

fn main() {
    let ctx = Arc::new(
        GpuContext::new_blocking().expect("Failed to init GPU"),
    );

    let size = 512;

    // Neon checkerboard: magenta + cyan tiles
    let checker = generate_cop(
        &ctx,
        &CheckerboardCop,
        &CheckerboardParams {
            color_a: [0.9, 0.1, 0.9, 1.0], // magenta
            color_b: [0.1, 0.9, 0.9, 1.0], // cyan
            frequency: [12.0, 12.0],
            width: size,
            height: size,
        },
    )
    .expect("checkerboard failed");

    // Radial gradient: bright center fading to dark edges
    let glow = generate_cop(
        &ctx,
        &RampCop,
        &RampParams {
            ramp_type: RampType::Radial,
            stops: vec![
                (0.0, [1.0, 1.0, 1.0, 1.0]),
                (0.6, [0.4, 0.2, 0.6, 1.0]),
                (1.0, [0.02, 0.01, 0.05, 1.0]),
            ],
            width: size,
            height: size,
        },
    )
    .expect("ramp failed");

    // Multiply checker with radial glow for vignette effect
    let composited = CompositeCop
        .execute(
            &ctx,
            &[&checker, &glow],
            &CompositeParams {
                operation: CompOp::Multiply,
                mix: 1.0,
            },
        )
        .expect("composite failed");

    // Swirl the whole thing for a trippy warp
    let swirled = SwirlCop
        .execute(
            &ctx,
            &[&composited],
            &SwirlParams {
                center: [0.5, 0.5],
                angle: 120.0,
                radius: 0.6,
            },
        )
        .expect("swirl failed");

    save_image(
        &swirled,
        &SaveImageParams {
            path: "neon_grid.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        },
    )
    .expect("save failed");

    println!("Wrote neon_grid.png ({}x{}) — neon checkerboard with radial glow + swirl", size, size);
}
