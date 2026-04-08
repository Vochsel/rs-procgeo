//! Procedural farmhouse built entirely from procgeo SOPs.
//!
//! Run:  cargo run -p procgeo --example farmhouse
//!
//! Outputs `farmhouse.obj` in the current directory.

use glam::Vec3;
use procgeo::prelude::*;
use procgeo_io::write_file;
use std::path::Path;

// ---------------------------------------------------------------------------
// Reusable building blocks
// ---------------------------------------------------------------------------

/// Rectangular prism centered at `center` with given `size`, colored.
fn colored_box(size: Vec3, center: Vec3, color: [f32; 3]) -> Result<Geometry, SopError> {
    generate(
        &BoxSop,
        &BoxParams {
            size,
            ..Default::default()
        },
    )?
    .apply(
        &TransformSop,
        &TransformParams {
            translate: center,
            ..Default::default()
        },
    )?
    .apply(&ColorSop, &ColorParams { color })
}

/// A triangular prism roof (wedge shape) built manually.
/// Runs along the X axis, centered at `center`.
fn roof_wedge(width: f32, depth: f32, peak: f32, center: Vec3) -> Geometry {
    let hw = width / 2.0;
    let hd = depth / 2.0;

    let mut geo = Geometry::with_capacity(6, 8);

    // Bottom-left-front, bottom-right-front, top-front (peak)
    let blf = geo.add_point(center + Vec3::new(-hw, 0.0, -hd));
    let brf = geo.add_point(center + Vec3::new(hw, 0.0, -hd));
    let tf = geo.add_point(center + Vec3::new(0.0, peak, -hd));

    // Bottom-left-back, bottom-right-back, top-back (peak)
    let blb = geo.add_point(center + Vec3::new(-hw, 0.0, hd));
    let brb = geo.add_point(center + Vec3::new(hw, 0.0, hd));
    let tb = geo.add_point(center + Vec3::new(0.0, peak, hd));

    // Front triangle
    geo.add_face(&[blf, brf, tf]);
    // Back triangle (reversed winding)
    geo.add_face(&[brb, blb, tb]);
    // Left slope
    geo.add_face(&[blb, blf, tf, tb]);
    // Right slope
    geo.add_face(&[brf, brb, tb, tf]);
    // Bottom
    geo.add_face(&[blf, blb, brb, brf]);

    geo
}

/// Cylinder column (e.g. for a chimney or fence post).
fn column(radius: f32, height: f32, center: Vec3, color: [f32; 3]) -> Result<Geometry, SopError> {
    generate(
        &TubeSop,
        &TubeParams {
            radius_bottom: radius,
            radius_top: radius,
            height,
            cols: 8,
            caps: TubeCap::Both,
            ..Default::default()
        },
    )?
    .apply(
        &TransformSop,
        &TransformParams {
            translate: center,
            ..Default::default()
        },
    )?
    .apply(&ColorSop, &ColorParams { color })
}

// ---------------------------------------------------------------------------
// Farmhouse assembly
// ---------------------------------------------------------------------------

fn build_farmhouse() -> Result<Geometry, SopError> {
    // -- Colors ---------------------------------------------------------
    let wall_color = [0.85, 0.82, 0.75]; // warm off-white
    let roof_color = [0.45, 0.25, 0.15]; // dark brown
    let door_color = [0.55, 0.30, 0.15]; // medium brown
    let window_color = [0.55, 0.75, 0.90]; // pale blue
    let chimney_color = [0.50, 0.40, 0.35]; // stone grey-brown
    let porch_color = [0.70, 0.60, 0.45]; // wood tan
    let ground_color = [0.35, 0.55, 0.25]; // grass green
    let fence_color = [0.80, 0.75, 0.60]; // pale wood

    // -- Main house body ------------------------------------------------
    let house_w = 6.0;
    let house_h = 3.0;
    let house_d = 8.0;
    let house = colored_box(
        Vec3::new(house_w, house_h, house_d),
        Vec3::new(0.0, house_h / 2.0, 0.0),
        wall_color,
    )?;

    // -- Roof -----------------------------------------------------------
    let roof_overhang = 0.6;
    let roof = roof_wedge(
        house_w + roof_overhang * 2.0,
        house_d + roof_overhang,
        2.0,
        Vec3::new(0.0, house_h, 0.0),
    )
    .apply(&ColorSop, &ColorParams { color: roof_color })?;

    // -- Front door -----------------------------------------------------
    let door = colored_box(
        Vec3::new(1.0, 2.2, 0.1),
        Vec3::new(0.0, 1.1, -(house_d / 2.0 + 0.05)),
        door_color,
    )?;

    // -- Windows (two flanking the door) --------------------------------
    let window_l = colored_box(
        Vec3::new(0.9, 0.9, 0.1),
        Vec3::new(-1.8, 1.8, -(house_d / 2.0 + 0.05)),
        window_color,
    )?;
    let window_r = colored_box(
        Vec3::new(0.9, 0.9, 0.1),
        Vec3::new(1.8, 1.8, -(house_d / 2.0 + 0.05)),
        window_color,
    )?;

    // -- Side windows ---------------------------------------------------
    let side_win_1 = colored_box(
        Vec3::new(0.1, 0.9, 0.9),
        Vec3::new(-(house_w / 2.0 + 0.05), 1.8, -1.5),
        window_color,
    )?;
    let side_win_2 = colored_box(
        Vec3::new(0.1, 0.9, 0.9),
        Vec3::new(-(house_w / 2.0 + 0.05), 1.8, 1.5),
        window_color,
    )?;

    // -- Chimney --------------------------------------------------------
    let chimney = colored_box(
        Vec3::new(0.8, 2.5, 0.8),
        Vec3::new(2.0, house_h + 1.25, 1.5),
        chimney_color,
    )?;

    // -- Porch (flat slab + two posts) ----------------------------------
    let porch_slab = colored_box(
        Vec3::new(4.0, 0.15, 1.5),
        Vec3::new(0.0, 0.075, -(house_d / 2.0 + 0.75)),
        porch_color,
    )?;
    let porch_roof = colored_box(
        Vec3::new(4.4, 0.1, 1.8),
        Vec3::new(0.0, 2.6, -(house_d / 2.0 + 0.75)),
        roof_color,
    )?;
    let post_l = column(
        0.08,
        2.5,
        Vec3::new(-1.8, 1.25, -(house_d / 2.0 + 1.4)),
        porch_color,
    )?;
    let post_r = column(
        0.08,
        2.5,
        Vec3::new(1.8, 1.25, -(house_d / 2.0 + 1.4)),
        porch_color,
    )?;

    // -- Ground plane ---------------------------------------------------
    let ground = generate(
        &GridSop,
        &GridParams {
            size: [30.0, 30.0],
            rows: 2,
            cols: 2,
            ..Default::default()
        },
    )?
    .apply(
        &ColorSop,
        &ColorParams {
            color: ground_color,
        },
    )?;

    // -- Fence posts around the yard ------------------------------------
    let fence_post = colored_box(Vec3::new(0.1, 0.8, 0.1), Vec3::ZERO, fence_color)?;

    // Create perimeter points: place posts at regular intervals
    let mut fence_points = Geometry::new();
    let spacing = 2.0;
    let half = 10.0;
    let mut i = -half;
    while i <= half {
        // Front and back edges
        fence_points.add_point(Vec3::new(i, 0.4, -half));
        fence_points.add_point(Vec3::new(i, 0.4, half));
        // Left and right edges (skip corners to avoid duplicates)
        if i > -half && i < half {
            fence_points.add_point(Vec3::new(-half, 0.4, i));
            fence_points.add_point(Vec3::new(half, 0.4, i));
        }
        i += spacing;
    }

    let fence = CopyToPointsSop.execute(
        &[&fence_post, &fence_points],
        &CopyToPointsParams {
            piece_attrib: String::new(),
        },
    )?;

    // -- Merge everything -----------------------------------------------
    let farmhouse = MergeSop.execute(
        &[
            &house,
            &roof,
            &door,
            &window_l,
            &window_r,
            &side_win_1,
            &side_win_2,
            &chimney,
            &porch_slab,
            &porch_roof,
            &post_l,
            &post_r,
            &ground,
            &fence,
        ],
        &MergeParams,
    )?;

    // Compute normals for the whole scene
    farmhouse.apply(&NormalSop, &NormalParams)
}

fn main() {
    match build_farmhouse() {
        Ok(geo) => {
            let path = Path::new("farmhouse.obj");
            write_file(&geo, path).expect("Failed to write OBJ");
            println!(
                "Wrote farmhouse.obj — {} points, {} prims",
                geo.num_points(),
                geo.num_prims(),
            );
        }
        Err(e) => eprintln!("Error building farmhouse: {e}"),
    }
}
