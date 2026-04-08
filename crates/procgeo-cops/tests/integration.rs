use procgeo_cops::prelude::*;
use procgeo_cops::registry::default_cop_registry;
use std::sync::Arc;

fn try_ctx() -> Option<Arc<GpuContext>> {
    GpuContext::new_blocking().ok().map(Arc::new)
}

#[test]
fn full_chain_generate_filter_composite_save() {
    let Some(ctx) = try_ctx() else {
        eprintln!("Skipping — no GPU");
        return;
    };

    let noise = generate_cop(
        &ctx,
        &procgeo_cops::generator::NoiseCop,
        &procgeo_cops::generator::NoiseParams {
            noise_type: procgeo_cops::generator::NoiseType::Simplex,
            frequency: 4.0,
            width: 64,
            height: 64,
            ..Default::default()
        },
    )
    .unwrap();

    let blurred = noise
        .apply(
            &procgeo_cops::filter::BlurCop,
            &procgeo_cops::filter::BlurParams {
                blur_type: procgeo_cops::filter::BlurType::Gaussian,
                radius_x: 2.0,
                radius_y: 2.0,
            },
        )
        .unwrap();
    let swirled = blurred
        .apply(
            &procgeo_cops::filter::SwirlCop,
            &procgeo_cops::filter::SwirlParams {
                angle: 45.0,
                radius: 0.5,
                ..Default::default()
            },
        )
        .unwrap();

    let checker = generate_cop(
        &ctx,
        &procgeo_cops::generator::CheckerboardCop,
        &procgeo_cops::generator::CheckerboardParams {
            frequency: [8.0, 8.0],
            width: 64,
            height: 64,
            ..Default::default()
        },
    )
    .unwrap();

    let result = procgeo_cops::composite::CompositeCop
        .execute(
            &ctx,
            &[&swirled, &checker],
            &procgeo_cops::composite::CompositeParams {
                operation: procgeo_cops::composite::CompOp::Multiply,
                mix: 1.0,
            },
        )
        .unwrap();

    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 64);
    let pixels = result.to_cpu().unwrap();
    assert_eq!(pixels.len(), 64 * 64 * 4);

    let tmp = std::env::temp_dir().join("procgeo_integration_test.png");
    let save_params = procgeo_cops::io::SaveImageParams {
        path: tmp.to_str().unwrap().to_string(),
        ..Default::default()
    };
    save_image(&result, &save_params).unwrap();
    assert!(std::fs::metadata(&tmp).unwrap().len() > 0);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn registry_round_trip() {
    let Some(ctx) = try_ctx() else {
        return;
    };
    let registry = default_cop_registry();
    let img = registry
        .execute(
            "constant",
            &ctx,
            &[],
            r#"{"color":[0.5,0.5,0.5,1.0],"width":8,"height":8}"#,
        )
        .unwrap();
    let flipped = registry
        .execute("flip", &ctx, &[&img], r#"{"horizontal":true}"#)
        .unwrap();
    assert_eq!(flipped.width(), 8);
}

#[test]
fn all_cops_listed() {
    let registry = default_cop_registry();
    let names = registry.list();
    for expected in [
        "constant",
        "checkerboard",
        "noise",
        "ramp",
        "load_image",
        "flip",
        "mirror",
        "channel_swap",
        "blur",
        "swirl",
        "rotate",
        "resize",
        "composite",
        "custom_shader",
    ] {
        assert!(names.contains(&expected), "missing COP: {expected}");
    }
}
