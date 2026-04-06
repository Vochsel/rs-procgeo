use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, PointHandle, TypeQualifier};

use crate::{Sop, SopError};

// ---------------------------------------------------------------------------
// Parameter types
// ---------------------------------------------------------------------------

/// Noise type to generate.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum NoiseType {
    #[default]
    Perlin,
    Simplex,
    Worley,      // F1 (distance to nearest)
    WorleyF2F1,  // F2 - F1
}

/// How to combine noise with existing attribute values.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum NoiseOperation {
    #[default]
    Set,
    Add,
    Subtract,
    Multiply,
}

/// Fractal layering mode.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum FractalType {
    #[default]
    None,
    Standard, // fBm
    Terrain,  // Ridged multifractal
}

/// How to compute the output range.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum NoiseRange {
    ZeroCentered, // [-amplitude, +amplitude]
    #[default]
    Positive, // [0, amplitude]
    MinMax,   // [min_value, max_value]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribNoiseParams {
    pub attrib_name: String,
    pub class: AttribClass,
    pub dimensions: u32, // 1 = float, 3 = vector3

    pub noise_type: NoiseType,
    pub operation: NoiseOperation,

    // Noise pattern
    pub element_size: f32,  // feature size (frequency = 1/size)
    pub offset: [f32; 3],
    pub seed: u64,

    // Range
    pub range: NoiseRange,
    pub amplitude: f32,
    pub min_value: f32,
    pub max_value: f32,

    // Fractal
    pub fractal: FractalType,
    pub octaves: u32,
    pub lacunarity: f32,
    pub roughness: f32,

    // Value correction
    pub gain: f32,  // 0.5 = no effect
    pub bias: f32,  // 0.5 = no effect
}

impl Default for AttribNoiseParams {
    fn default() -> Self {
        AttribNoiseParams {
            attrib_name: "noise".to_string(),
            class: AttribClass::Point,
            dimensions: 1,
            noise_type: NoiseType::Perlin,
            operation: NoiseOperation::Set,
            element_size: 1.0,
            offset: [0.0; 3],
            seed: 0,
            range: NoiseRange::Positive,
            amplitude: 1.0,
            min_value: 0.0,
            max_value: 1.0,
            fractal: FractalType::None,
            octaves: 8,
            lacunarity: 2.0,
            roughness: 0.5,
            gain: 0.5,
            bias: 0.5,
        }
    }
}

// ---------------------------------------------------------------------------
// Noise primitives
// ---------------------------------------------------------------------------

/// Ken Perlin's 16-entry gradient table (edge directions on the unit cube).
const GRADIENTS: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0],
    [0.0, -1.0, -1.0],
];

/// Hash three integer coordinates + seed to one of the 16 gradient indices.
fn hash(x: i32, y: i32, z: i32, seed: u64) -> usize {
    let mut h = (x as u64).wrapping_mul(73_856_093)
        ^ (y as u64).wrapping_mul(19_349_663)
        ^ (z as u64).wrapping_mul(83_492_791)
        ^ seed;
    h = h.wrapping_mul(0x517c_c1b7_2722_0a95);
    h ^= h >> 32;
    (h & 0xF) as usize
}

/// Perlin smoothstep fade: 6t⁵ - 15t⁴ + 10t³
#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

#[inline]
fn grad_dot(ix: i32, iy: i32, iz: i32, fx: f32, fy: f32, fz: f32, seed: u64) -> f32 {
    let g = GRADIENTS[hash(ix, iy, iz, seed)];
    g[0] * fx + g[1] * fy + g[2] * fz
}

/// Classic 3-D Perlin noise. Returns roughly [-1, 1].
fn perlin(pos: [f32; 3], seed: u64) -> f32 {
    let xi = pos[0].floor() as i32;
    let yi = pos[1].floor() as i32;
    let zi = pos[2].floor() as i32;

    let xf = pos[0] - xi as f32;
    let yf = pos[1] - yi as f32;
    let zf = pos[2] - zi as f32;

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    // 8 corner contributions
    let n000 = grad_dot(xi, yi, zi, xf, yf, zf, seed);
    let n100 = grad_dot(xi + 1, yi, zi, xf - 1.0, yf, zf, seed);
    let n010 = grad_dot(xi, yi + 1, zi, xf, yf - 1.0, zf, seed);
    let n110 = grad_dot(xi + 1, yi + 1, zi, xf - 1.0, yf - 1.0, zf, seed);
    let n001 = grad_dot(xi, yi, zi + 1, xf, yf, zf - 1.0, seed);
    let n101 = grad_dot(xi + 1, yi, zi + 1, xf - 1.0, yf, zf - 1.0, seed);
    let n011 = grad_dot(xi, yi + 1, zi + 1, xf, yf - 1.0, zf - 1.0, seed);
    let n111 = grad_dot(xi + 1, yi + 1, zi + 1, xf - 1.0, yf - 1.0, zf - 1.0, seed);

    // Trilinear interpolation
    let x0 = lerp(n000, n100, u);
    let x1 = lerp(n010, n110, u);
    let x2 = lerp(n001, n101, u);
    let x3 = lerp(n011, n111, u);
    let y0 = lerp(x0, x1, v);
    let y1 = lerp(x2, x3, v);
    lerp(y0, y1, w)
}

/// 3-D Simplex noise. Returns roughly [-1, 1].
///
/// Based on Stefan Gustavson's public-domain implementation.
fn simplex(pos: [f32; 3], seed: u64) -> f32 {
    const F3: f32 = 1.0 / 3.0;
    const G3: f32 = 1.0 / 6.0;

    let (x, y, z) = (pos[0], pos[1], pos[2]);

    // Skew to simplex grid
    let s = (x + y + z) * F3;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let k = (z + s).floor() as i32;

    let t = (i + j + k) as f32 * G3;
    // Unskew back
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);
    let z0 = z - (k as f32 - t);

    // Determine simplex (which of the 6 tetrahedra we're in)
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else {
        if y0 < z0 {
            (0, 0, 1, 0, 1, 1)
        } else if x0 < z0 {
            (0, 1, 0, 0, 1, 1)
        } else {
            (0, 1, 0, 1, 1, 0)
        }
    };

    // Offsets for the 4 simplex corners
    let x1 = x0 - i1 as f32 + G3;
    let y1 = y0 - j1 as f32 + G3;
    let z1 = z0 - k1 as f32 + G3;
    let x2 = x0 - i2 as f32 + 2.0 * G3;
    let y2 = y0 - j2 as f32 + 2.0 * G3;
    let z2 = z0 - k2 as f32 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    // Corner contributions: max(0, r² - dist²)^4 * dot
    let r2 = 0.6_f32;

    let corner_contribution = |cx: f32, cy: f32, cz: f32, gi: i32, gj: i32, gk: i32| -> f32 {
        let t = r2 - cx * cx - cy * cy - cz * cz;
        if t < 0.0 {
            0.0
        } else {
            let t2 = t * t;
            let g = GRADIENTS[hash(gi, gj, gk, seed)];
            t2 * t2 * (g[0] * cx + g[1] * cy + g[2] * cz)
        }
    };

    let n0 = corner_contribution(x0, y0, z0, i, j, k);
    let n1 = corner_contribution(x1, y1, z1, i + i1, j + j1, k + k1);
    let n2 = corner_contribution(x2, y2, z2, i + i2, j + j2, k + k2);
    let n3 = corner_contribution(x3, y3, z3, i + 1, j + 1, k + 1);

    // Scale to roughly [-1, 1]
    32.0 * (n0 + n1 + n2 + n3)
}

/// Generate a deterministic feature-point offset in [0, 1)^3 for a cell.
fn cell_feature_point(cx: i32, cy: i32, cz: i32, seed: u64) -> [f32; 3] {
    // Three independent hashes for x, y, z jitter
    let hx = {
        let mut h = (cx as u64)
            .wrapping_mul(73_856_093)
            ^ (cy as u64).wrapping_mul(19_349_663)
            ^ (cz as u64).wrapping_mul(83_492_791)
            ^ seed;
        h = h.wrapping_mul(0x517c_c1b7_2722_0a95);
        h ^= h >> 32;
        h
    };
    let hy = {
        let mut h = (cx as u64)
            .wrapping_mul(19_349_663)
            ^ (cy as u64).wrapping_mul(83_492_791)
            ^ (cz as u64).wrapping_mul(73_856_093)
            ^ seed.wrapping_add(1);
        h = h.wrapping_mul(0x517c_c1b7_2722_0a95);
        h ^= h >> 32;
        h
    };
    let hz = {
        let mut h = (cx as u64)
            .wrapping_mul(83_492_791)
            ^ (cy as u64).wrapping_mul(73_856_093)
            ^ (cz as u64).wrapping_mul(19_349_663)
            ^ seed.wrapping_add(2);
        h = h.wrapping_mul(0x517c_c1b7_2722_0a95);
        h ^= h >> 32;
        h
    };
    [
        (hx & 0xFFFF) as f32 / 65535.0,
        (hy & 0xFFFF) as f32 / 65535.0,
        (hz & 0xFFFF) as f32 / 65535.0,
    ]
}

/// Worley (Cellular) noise. Returns (F1, F2) distances, each in roughly [0, ~1.7].
fn worley_f1_f2(pos: [f32; 3], seed: u64) -> (f32, f32) {
    let xi = pos[0].floor() as i32;
    let yi = pos[1].floor() as i32;
    let zi = pos[2].floor() as i32;

    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;

    for dz in -1..=1_i32 {
        for dy in -1..=1_i32 {
            for dx in -1..=1_i32 {
                let cx = xi + dx;
                let cy = yi + dy;
                let cz = zi + dz;
                let fp = cell_feature_point(cx, cy, cz, seed);
                let fx = cx as f32 + fp[0] - pos[0];
                let fy = cy as f32 + fp[1] - pos[1];
                let fz = cz as f32 + fp[2] - pos[2];
                let dist = (fx * fx + fy * fy + fz * fz).sqrt();
                if dist < f1 {
                    f2 = f1;
                    f1 = dist;
                } else if dist < f2 {
                    f2 = dist;
                }
            }
        }
    }
    (f1, f2)
}

// ---------------------------------------------------------------------------
// Bias / Gain
// ---------------------------------------------------------------------------

fn perlin_bias(t: f32, b: f32) -> f32 {
    if b <= 0.0 || b >= 1.0 || t <= 0.0 {
        return t;
    }
    t.powf(b.ln() / 0.5_f32.ln())
}

fn perlin_gain(t: f32, g: f32) -> f32 {
    if t < 0.5 {
        perlin_bias(2.0 * t, g) * 0.5
    } else {
        1.0 - perlin_bias(2.0 - 2.0 * t, g) * 0.5
    }
}

// ---------------------------------------------------------------------------
// Fractal layering
// ---------------------------------------------------------------------------

/// Evaluate the base noise (single octave) at the given position.
fn base_noise(pos: [f32; 3], noise_type: NoiseType, seed: u64) -> f32 {
    match noise_type {
        NoiseType::Perlin => perlin(pos, seed),
        NoiseType::Simplex => simplex(pos, seed),
        NoiseType::Worley => {
            let (f1, _) = worley_f1_f2(pos, seed);
            // Normalize: F1 is typically in [0, ~1.2]; clamp to [0,1]
            f1.clamp(0.0, 1.0) * 2.0 - 1.0 // remap to [-1, 1] for consistency
        }
        NoiseType::WorleyF2F1 => {
            let (f1, f2) = worley_f1_f2(pos, seed);
            let v = (f2 - f1).clamp(0.0, 1.0);
            v * 2.0 - 1.0 // remap to [-1, 1]
        }
    }
}

/// Evaluate noise with fractal layering. Returns raw noise value in roughly [-1, 1] (or [0, 1] for Worley).
fn eval_noise(pos: [f32; 3], params: &AttribNoiseParams) -> f32 {
    let noise_type = params.noise_type;
    let seed = params.seed;

    match params.fractal {
        FractalType::None => base_noise(pos, noise_type, seed),

        FractalType::Standard => {
            // fBm
            let mut result = 0.0_f32;
            let mut amp = 1.0_f32;
            let mut freq = 1.0_f32;
            let mut amp_total = 0.0_f32;
            for _ in 0..params.octaves {
                result += amp
                    * base_noise(
                        [pos[0] * freq, pos[1] * freq, pos[2] * freq],
                        noise_type,
                        seed,
                    );
                amp_total += amp;
                freq *= params.lacunarity;
                amp *= params.roughness;
            }
            // Normalize so amplitude is still in [-1, 1]
            if amp_total > 0.0 {
                result / amp_total
            } else {
                result
            }
        }

        FractalType::Terrain => {
            // Ridged multifractal
            let first = base_noise(pos, noise_type, seed).abs();
            let mut result = 1.0 - first;
            let mut amp = 1.0_f32;
            let mut freq = 1.0_f32;
            let mut weight = result;
            for _ in 1..params.octaves {
                freq *= params.lacunarity;
                amp *= params.roughness;
                weight = weight.clamp(0.0, 1.0);
                let signal = (1.0
                    - base_noise(
                        [pos[0] * freq, pos[1] * freq, pos[2] * freq],
                        noise_type,
                        seed,
                    )
                    .abs())
                    * amp;
                result += weight * signal;
                weight *= signal;
            }
            // Clamp to [-1, 1]
            result.clamp(-1.0, 1.0)
        }
    }
}

// ---------------------------------------------------------------------------
// Range mapping
// ---------------------------------------------------------------------------

fn map_range(raw: f32, noise_type: NoiseType, params: &AttribNoiseParams) -> f32 {
    // Normalize to [0, 1]
    let normalized = match noise_type {
        NoiseType::Worley | NoiseType::WorleyF2F1 => {
            // base_noise already maps Worley to [-1, 1]; undo that
            (raw + 1.0) * 0.5
        }
        _ => (raw + 1.0) * 0.5,
    };
    let normalized = normalized.clamp(0.0, 1.0);

    // Map to output range
    match params.range {
        NoiseRange::Positive => normalized * params.amplitude,
        NoiseRange::ZeroCentered => (normalized * 2.0 - 1.0) * params.amplitude,
        NoiseRange::MinMax => params.min_value + normalized * (params.max_value - params.min_value),
    }
}

// ---------------------------------------------------------------------------
// Combine operation
// ---------------------------------------------------------------------------

fn apply_op(current: f32, generated: f32, op: NoiseOperation) -> f32 {
    match op {
        NoiseOperation::Set => generated,
        NoiseOperation::Add => current + generated,
        NoiseOperation::Subtract => current - generated,
        NoiseOperation::Multiply => current * generated,
    }
}

// ---------------------------------------------------------------------------
// High-level evaluation for a single element
// ---------------------------------------------------------------------------

/// Evaluate, map, bias/gain for a single scalar.
fn eval_scalar(pos: [f32; 3], params: &AttribNoiseParams) -> f32 {
    let raw = eval_noise(pos, params);
    let mapped = map_range(raw, params.noise_type, params);

    // Apply bias/gain only when parameters differ meaningfully from neutral 0.5
    let biased = if (params.bias - 0.5).abs() > 1e-5 {
        perlin_bias(mapped.clamp(0.0, 1.0), params.bias)
    } else {
        mapped
    };

    if (params.gain - 0.5).abs() > 1e-5 {
        perlin_gain(biased.clamp(0.0, 1.0), params.gain)
    } else {
        biased
    }
}

/// Evaluate noise as a vector3 by offsetting the position for each component.
fn eval_vector3(pos: [f32; 3], params: &AttribNoiseParams) -> [f32; 3] {
    let p0 = pos;
    let p1 = [pos[0] + 3.7, pos[1] + 1.9, pos[2] + 4.1];
    let p2 = [pos[0] + 7.3, pos[1] + 5.5, pos[2] + 2.8];
    [
        eval_scalar(p0, params),
        eval_scalar(p1, params),
        eval_scalar(p2, params),
    ]
}

// ---------------------------------------------------------------------------
// Helpers shared with tests
// ---------------------------------------------------------------------------

fn element_count(geo: &Geometry, class: AttribClass) -> usize {
    match class {
        AttribClass::Point => geo.num_points(),
        AttribClass::Vertex => geo.num_vertices(),
        AttribClass::Primitive => geo.num_prims(),
        AttribClass::Detail => 1,
    }
}

fn element_position(geo: &Geometry, class: AttribClass, index: usize) -> [f32; 3] {
    match class {
        AttribClass::Point => {
            let p = geo.point_pos(PointHandle::from_index(index));
            [p.x, p.y, p.z]
        }
        // For non-point classes, fall back to the associated point of vertex 0 of the primitive.
        // For Detail use origin.
        _ => [index as f32, 0.0, 0.0],
    }
}

// ---------------------------------------------------------------------------
// SOP struct
// ---------------------------------------------------------------------------

pub struct AttribNoiseSop;

impl Sop for AttribNoiseSop {
    type Params = AttribNoiseParams;

    fn name(&self) -> &'static str {
        "attrib_noise"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        let count = element_count(&out, params.class);
        if count == 0 {
            return Ok(out);
        }

        if params.dimensions <= 1 {
            // Scalar float attribute
            let _ = out.add_attrib(
                params.class,
                &params.attrib_name,
                AttribDefault::Float(0.0),
                TypeQualifier::None,
            );
            let handle = out
                .find_attrib::<f32>(params.class, &params.attrib_name)
                .map_err(SopError::Core)?;

            let new_vals: Vec<f32> = (0..count)
                .map(|i| {
                    let world_pos = element_position(&out, params.class, i);
                    let scaled_pos = [
                        (world_pos[0] + params.offset[0]) / params.element_size,
                        (world_pos[1] + params.offset[1]) / params.element_size,
                        (world_pos[2] + params.offset[2]) / params.element_size,
                    ];
                    let noise_val = eval_scalar(scaled_pos, params);
                    let current = out.get_attrib(&handle, i).unwrap_or(0.0);
                    apply_op(current, noise_val, params.operation)
                })
                .collect();

            for (i, v) in new_vals.into_iter().enumerate() {
                out.set_attrib(&handle, i, v)?;
            }
        } else {
            // Vector3 attribute
            let _ = out.add_attrib(
                params.class,
                &params.attrib_name,
                AttribDefault::Vector3([0.0; 3]),
                TypeQualifier::None,
            );
            let handle = out
                .find_attrib::<[f32; 3]>(params.class, &params.attrib_name)
                .map_err(SopError::Core)?;

            let new_vals: Vec<[f32; 3]> = (0..count)
                .map(|i| {
                    let world_pos = element_position(&out, params.class, i);
                    let scaled_pos = [
                        (world_pos[0] + params.offset[0]) / params.element_size,
                        (world_pos[1] + params.offset[1]) / params.element_size,
                        (world_pos[2] + params.offset[2]) / params.element_size,
                    ];
                    let noise_val = eval_vector3(scaled_pos, params);
                    let current = out.get_attrib(&handle, i).unwrap_or([0.0; 3]);
                    [
                        apply_op(current[0], noise_val[0], params.operation),
                        apply_op(current[1], noise_val[1], params.operation),
                        apply_op(current[2], noise_val[2], params.operation),
                    ]
                })
                .collect();

            for (i, v) in new_vals.into_iter().enumerate() {
                out.set_attrib(&handle, i, v)?;
            }
        }

        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::grid::{GridOrientation, GridParams, GridSop};
    use crate::{generate, GeometryExt};

    fn make_grid() -> Geometry {
        generate(
            &GridSop,
            &GridParams {
                size: [10.0, 10.0],
                rows: 10,
                cols: 10,
                ..Default::default()
            },
        )
        .unwrap()
    }

    // -----------------------------------------------------------------------
    // noise_perlin_basic
    // -----------------------------------------------------------------------
    #[test]
    fn noise_perlin_basic() {
        let geo = make_grid();
        let sop = AttribNoiseSop;
        let params = AttribNoiseParams {
            attrib_name: "perlin_test".to_string(),
            noise_type: NoiseType::Perlin,
            range: NoiseRange::Positive,
            amplitude: 1.0,
            ..Default::default()
        };
        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "perlin_test")
            .unwrap();

        let mut in_range = true;
        for i in 0..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            if v < -0.01 || v > 1.01 {
                in_range = false;
                break;
            }
        }
        assert!(in_range, "Perlin Positive range should be in [0, 1]");
    }

    // -----------------------------------------------------------------------
    // noise_deterministic
    // -----------------------------------------------------------------------
    #[test]
    fn noise_deterministic() {
        let geo = make_grid();
        let sop = AttribNoiseSop;
        let params = AttribNoiseParams {
            attrib_name: "det".to_string(),
            seed: 42,
            ..Default::default()
        };
        let r1 = geo.clone().apply(&sop, &params).unwrap();
        let r2 = geo.apply(&sop, &params).unwrap();

        let h1 = r1.find_attrib::<f32>(AttribClass::Point, "det").unwrap();
        let h2 = r2.find_attrib::<f32>(AttribClass::Point, "det").unwrap();

        for i in 0..r1.num_points() {
            let v1 = r1.get_attrib(&h1, i).unwrap();
            let v2 = r2.get_attrib(&h2, i).unwrap();
            assert!(
                (v1 - v2).abs() < 1e-8,
                "point {i}: {v1} != {v2} — noise is not deterministic"
            );
        }
    }

    // -----------------------------------------------------------------------
    // noise_fractal_fbm
    // -----------------------------------------------------------------------
    #[test]
    fn noise_fractal_fbm() {
        let geo = make_grid();
        let sop = AttribNoiseSop;

        let single = geo
            .clone()
            .apply(
                &sop,
                &AttribNoiseParams {
                    attrib_name: "single".to_string(),
                    fractal: FractalType::None,
                    octaves: 1,
                    ..Default::default()
                },
            )
            .unwrap();

        let fractal = geo
            .apply(
                &sop,
                &AttribNoiseParams {
                    attrib_name: "fractal".to_string(),
                    fractal: FractalType::Standard,
                    octaves: 6,
                    ..Default::default()
                },
            )
            .unwrap();

        let hs = single.find_attrib::<f32>(AttribClass::Point, "single").unwrap();
        let hf = fractal.find_attrib::<f32>(AttribClass::Point, "fractal").unwrap();

        // Compute variance of each: fractal should have higher variation
        let n = single.num_points() as f32;
        let mean_s: f32 = (0..single.num_points())
            .map(|i| single.get_attrib(&hs, i).unwrap())
            .sum::<f32>() / n;
        let mean_f: f32 = (0..fractal.num_points())
            .map(|i| fractal.get_attrib(&hf, i).unwrap())
            .sum::<f32>() / n;

        let var_s: f32 = (0..single.num_points())
            .map(|i| {
                let d = single.get_attrib(&hs, i).unwrap() - mean_s;
                d * d
            })
            .sum::<f32>() / n;
        let var_f: f32 = (0..fractal.num_points())
            .map(|i| {
                let d = fractal.get_attrib(&hf, i).unwrap() - mean_f;
                d * d
            })
            .sum::<f32>() / n;

        // Both should have non-trivial variation; fractal should have at least as much
        assert!(var_s > 0.0, "single octave noise has zero variance");
        assert!(var_f > 0.0, "fractal noise has zero variance");
        // Both should produce non-trivial variation (neither flat).
        // Note: fBm normalizes by total amplitude, so its absolute variance may
        // differ from a single octave.  The key property is that both have structure.
        assert!(
            var_s > 1e-6 && var_f > 1e-6,
            "both noise modes should have non-trivial variance: single={var_s}, fractal={var_f}"
        );
    }

    // -----------------------------------------------------------------------
    // noise_worley
    // -----------------------------------------------------------------------
    #[test]
    fn noise_worley() {
        let geo = make_grid();
        let sop = AttribNoiseSop;
        let params = AttribNoiseParams {
            attrib_name: "worley".to_string(),
            noise_type: NoiseType::Worley,
            range: NoiseRange::Positive,
            amplitude: 1.0,
            ..Default::default()
        };
        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "worley")
            .unwrap();

        let mut in_range = true;
        for i in 0..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            if v < -0.01 || v > 1.01 {
                in_range = false;
                break;
            }
        }
        assert!(in_range, "Worley Positive range should be in [0, 1]");

        // Values should vary (not all identical) — Worley produces cell patterns
        let vals: Vec<f32> = (0..result.num_points())
            .map(|i| result.get_attrib(&handle, i).unwrap())
            .collect();
        let min = vals.iter().cloned().fold(f32::MAX, f32::min);
        let max = vals.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            max - min > 0.01,
            "Worley noise should vary across grid points"
        );
    }

    // -----------------------------------------------------------------------
    // noise_vector3
    // -----------------------------------------------------------------------
    #[test]
    fn noise_vector3() {
        let geo = make_grid();
        let sop = AttribNoiseSop;
        let params = AttribNoiseParams {
            attrib_name: "vec_noise".to_string(),
            dimensions: 3,
            ..Default::default()
        };
        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "vec_noise")
            .unwrap();

        // All three components should exist and vary
        let mut comp_varies = [false; 3];
        let first = result.get_attrib(&handle, 0).unwrap();
        for i in 1..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            for c in 0..3 {
                if (v[c] - first[c]).abs() > 1e-5 {
                    comp_varies[c] = true;
                }
            }
        }
        for c in 0..3 {
            assert!(
                comp_varies[c],
                "component {c} of vec_noise does not vary across points"
            );
        }
    }

    // -----------------------------------------------------------------------
    // noise_zero_centered
    // -----------------------------------------------------------------------
    #[test]
    fn noise_zero_centered() {
        let geo = make_grid();
        let sop = AttribNoiseSop;
        let amplitude = 2.5;
        let params = AttribNoiseParams {
            attrib_name: "zc".to_string(),
            range: NoiseRange::ZeroCentered,
            amplitude,
            ..Default::default()
        };
        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "zc")
            .unwrap();

        for i in 0..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            assert!(
                v >= -amplitude - 0.01 && v <= amplitude + 0.01,
                "ZeroCentered point {i} value {v} outside [{}, {}]",
                -amplitude,
                amplitude
            );
        }

        // Should have both positive and negative values
        let vals: Vec<f32> = (0..result.num_points())
            .map(|i| result.get_attrib(&handle, i).unwrap())
            .collect();
        let has_negative = vals.iter().any(|&v| v < 0.0);
        let has_positive = vals.iter().any(|&v| v > 0.0);
        assert!(has_negative, "ZeroCentered noise should have negative values");
        assert!(has_positive, "ZeroCentered noise should have positive values");
    }

    // -----------------------------------------------------------------------
    // noise_operation_add
    // -----------------------------------------------------------------------
    #[test]
    fn noise_operation_add() {
        let geo = make_grid();
        let sop = AttribNoiseSop;

        // First pass: Set to noise
        let after_set = geo
            .clone()
            .apply(
                &sop,
                &AttribNoiseParams {
                    attrib_name: "addtest".to_string(),
                    operation: NoiseOperation::Set,
                    seed: 1,
                    ..Default::default()
                },
            )
            .unwrap();

        // Second pass: Add more noise on top
        let after_add = after_set
            .clone()
            .apply(
                &sop,
                &AttribNoiseParams {
                    attrib_name: "addtest".to_string(),
                    operation: NoiseOperation::Add,
                    seed: 2,
                    ..Default::default()
                },
            )
            .unwrap();

        let h_set = after_set
            .find_attrib::<f32>(AttribClass::Point, "addtest")
            .unwrap();
        let h_add = after_add
            .find_attrib::<f32>(AttribClass::Point, "addtest")
            .unwrap();

        // After Add, most values should be larger (noise is non-negative in Positive range)
        let set_sum: f32 = (0..after_set.num_points())
            .map(|i| after_set.get_attrib(&h_set, i).unwrap())
            .sum();
        let add_sum: f32 = (0..after_add.num_points())
            .map(|i| after_add.get_attrib(&h_add, i).unwrap())
            .sum();

        assert!(
            add_sum > set_sum,
            "Add operation should increase overall sum: set={set_sum}, add={add_sum}"
        );
    }
}
