use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribType, Geometry, PointHandle, PrimHandle, Primitive};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceDirection {
    #[default]
    #[serde(alias = "normal")]
    Normal,
    #[serde(alias = "x")]
    X,
    #[serde(alias = "y")]
    Y,
    #[serde(alias = "z")]
    Z,
    #[serde(alias = "rgbToXyz", alias = "rgb_to_xyz", alias = "rgbtoxyz")]
    RGBToXYZ,
    #[serde(alias = "customVector", alias = "custom_vector")]
    CustomVector,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceCoordinates {
    #[default]
    #[serde(alias = "auto")]
    Auto,
    #[serde(alias = "uv")]
    UV,
    #[serde(alias = "boundingBox", alias = "bounding_box", alias = "bbox")]
    BoundingBox,
    #[serde(alias = "position", alias = "local")]
    Position,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceProjection {
    #[serde(alias = "xy")]
    XY,
    #[default]
    #[serde(alias = "xz")]
    XZ,
    #[serde(alias = "yz")]
    YZ,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceSampleChannel {
    #[default]
    #[serde(alias = "luminance", alias = "value")]
    Luminance,
    #[serde(alias = "red", alias = "r")]
    Red,
    #[serde(alias = "green", alias = "g")]
    Green,
    #[serde(alias = "blue", alias = "b")]
    Blue,
    #[serde(alias = "alpha", alias = "a")]
    Alpha,
    #[serde(alias = "average", alias = "avg")]
    Average,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceSampler {
    #[serde(alias = "nearest")]
    Nearest,
    #[default]
    #[serde(alias = "bilinear", alias = "linear")]
    Bilinear,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceWrapMode {
    #[serde(alias = "clamp")]
    Clamp,
    #[default]
    #[serde(alias = "repeat", alias = "tile")]
    Repeat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceNoiseType {
    #[default]
    #[serde(alias = "perlin")]
    Perlin,
    #[serde(alias = "simplex")]
    Simplex,
    #[serde(alias = "worley")]
    Worley,
    #[serde(alias = "worleyF2F1", alias = "worley_f2f1")]
    WorleyF2F1,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplaceNoiseFractal {
    #[default]
    #[serde(alias = "none")]
    None,
    #[serde(alias = "standard", alias = "fbm")]
    Standard,
    #[serde(alias = "terrain", alias = "ridged")]
    Terrain,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplaceTexture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<f32>,
}

impl DisplaceTexture {
    fn validate(&self) -> Result<(), SopError> {
        if self.width == 0 || self.height == 0 {
            return Err(SopError::InvalidParam(
                "texture width and height must both be > 0".to_string(),
            ));
        }

        let expected = self.width as usize * self.height as usize * 4;
        if self.pixels.len() != expected {
            return Err(SopError::InvalidParam(format!(
                "texture expects {expected} RGBA floats, got {}",
                self.pixels.len()
            )));
        }

        Ok(())
    }

    fn pixel(&self, x: usize, y: usize) -> [f32; 4] {
        let base = (y * self.width as usize + x) * 4;
        [
            self.pixels[base],
            self.pixels[base + 1],
            self.pixels[base + 2],
            self.pixels[base + 3],
        ]
    }

    fn sample(&self, uv: [f32; 2], sampler: DisplaceSampler, wrap: DisplaceWrapMode) -> [f32; 4] {
        if self.width == 1 && self.height == 1 {
            return self.pixel(0, 0);
        }

        match sampler {
            DisplaceSampler::Nearest => {
                let x = sample_axis_nearest(uv[0], self.width, wrap);
                let y = sample_axis_nearest(uv[1], self.height, wrap);
                self.pixel(x, y)
            }
            DisplaceSampler::Bilinear => {
                let (x0, x1, tx) = sample_axis_linear(uv[0], self.width, wrap);
                let (y0, y1, ty) = sample_axis_linear(uv[1], self.height, wrap);
                let c00 = self.pixel(x0, y0);
                let c10 = self.pixel(x1, y0);
                let c01 = self.pixel(x0, y1);
                let c11 = self.pixel(x1, y1);

                let mut out = [0.0; 4];
                for channel in 0..4 {
                    let top = lerp(c00[channel], c10[channel], tx);
                    let bottom = lerp(c01[channel], c11[channel], tx);
                    out[channel] = lerp(top, bottom, ty);
                }
                out
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplaceNoiseParams {
    pub noise_type: DisplaceNoiseType,
    pub fractal: DisplaceNoiseFractal,
    pub scale: [f32; 3],
    pub offset: [f32; 3],
    pub seed: u64,
    pub octaves: u32,
    pub lacunarity: f32,
    pub roughness: f32,
}

impl Default for DisplaceNoiseParams {
    fn default() -> Self {
        Self {
            noise_type: DisplaceNoiseType::Perlin,
            fractal: DisplaceNoiseFractal::None,
            scale: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            seed: 0,
            octaves: 4,
            lacunarity: 2.0,
            roughness: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplaceParams {
    pub strength: f32,
    #[serde(alias = "midLevel")]
    pub midlevel: f32,
    pub direction: DisplaceDirection,
    pub coordinates: DisplaceCoordinates,
    pub projection: DisplaceProjection,
    #[serde(alias = "uvAttrib")]
    pub uv_attrib: String,
    #[serde(alias = "normalAttrib")]
    pub normal_attrib: String,
    #[serde(alias = "sampleChannel")]
    pub sample_channel: DisplaceSampleChannel,
    pub sampler: DisplaceSampler,
    pub wrap: DisplaceWrapMode,
    #[serde(alias = "coordScale")]
    pub coord_scale: [f32; 2],
    #[serde(alias = "coordOffset")]
    pub coord_offset: [f32; 2],
    #[serde(alias = "customVector")]
    pub custom_vector: [f32; 3],
    pub texture: Option<DisplaceTexture>,
    pub noise: Option<DisplaceNoiseParams>,
}

impl Default for DisplaceParams {
    fn default() -> Self {
        Self {
            strength: 1.0,
            midlevel: 0.5,
            direction: DisplaceDirection::Normal,
            coordinates: DisplaceCoordinates::Auto,
            projection: DisplaceProjection::XZ,
            uv_attrib: "uv".to_string(),
            normal_attrib: "N".to_string(),
            sample_channel: DisplaceSampleChannel::Luminance,
            sampler: DisplaceSampler::Bilinear,
            wrap: DisplaceWrapMode::Repeat,
            coord_scale: [1.0, 1.0],
            coord_offset: [0.0, 0.0],
            custom_vector: [0.0, 1.0, 0.0],
            texture: None,
            noise: None,
        }
    }
}

pub struct DisplaceSop;

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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

fn wrap_repeat(value: f32) -> f32 {
    let wrapped = value - value.floor();
    if wrapped < 0.0 {
        wrapped + 1.0
    } else {
        wrapped
    }
}

fn sample_axis_nearest(coord: f32, size: u32, wrap: DisplaceWrapMode) -> usize {
    if size <= 1 {
        return 0;
    }

    match wrap {
        DisplaceWrapMode::Clamp => {
            let clamped = coord.clamp(0.0, 1.0);
            (clamped * (size - 1) as f32).round() as usize
        }
        DisplaceWrapMode::Repeat => {
            let wrapped = wrap_repeat(coord);
            let idx = (wrapped * size as f32).floor() as usize;
            idx.min(size as usize - 1)
        }
    }
}

fn sample_axis_linear(coord: f32, size: u32, wrap: DisplaceWrapMode) -> (usize, usize, f32) {
    if size <= 1 {
        return (0, 0, 0.0);
    }

    match wrap {
        DisplaceWrapMode::Clamp => {
            let pos = coord.clamp(0.0, 1.0) * (size - 1) as f32;
            let base = pos.floor() as usize;
            let next = (base + 1).min(size as usize - 1);
            (base, next, pos - base as f32)
        }
        DisplaceWrapMode::Repeat => {
            let pos = wrap_repeat(coord) * size as f32 - 0.5;
            let base = pos.floor();
            let t = pos - base;
            let size_i = size as i32;
            let x0 = base as i32;
            let x1 = x0 + 1;
            (
                x0.rem_euclid(size_i) as usize,
                x1.rem_euclid(size_i) as usize,
                t,
            )
        }
    }
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn hash(x: i32, y: i32, z: i32, seed: u64) -> usize {
    let mut h = (x as u64).wrapping_mul(73_856_093)
        ^ (y as u64).wrapping_mul(19_349_663)
        ^ (z as u64).wrapping_mul(83_492_791)
        ^ seed;
    h = h.wrapping_mul(0x517c_c1b7_2722_0a95);
    h ^= h >> 32;
    (h & 0xF) as usize
}

fn grad_dot(ix: i32, iy: i32, iz: i32, fx: f32, fy: f32, fz: f32, seed: u64) -> f32 {
    let g = GRADIENTS[hash(ix, iy, iz, seed)];
    g[0] * fx + g[1] * fy + g[2] * fz
}

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

    let n000 = grad_dot(xi, yi, zi, xf, yf, zf, seed);
    let n100 = grad_dot(xi + 1, yi, zi, xf - 1.0, yf, zf, seed);
    let n010 = grad_dot(xi, yi + 1, zi, xf, yf - 1.0, zf, seed);
    let n110 = grad_dot(xi + 1, yi + 1, zi, xf - 1.0, yf - 1.0, zf, seed);
    let n001 = grad_dot(xi, yi, zi + 1, xf, yf, zf - 1.0, seed);
    let n101 = grad_dot(xi + 1, yi, zi + 1, xf - 1.0, yf, zf - 1.0, seed);
    let n011 = grad_dot(xi, yi + 1, zi + 1, xf, yf - 1.0, zf - 1.0, seed);
    let n111 = grad_dot(xi + 1, yi + 1, zi + 1, xf - 1.0, yf - 1.0, zf - 1.0, seed);

    let x0 = lerp(n000, n100, u);
    let x1 = lerp(n010, n110, u);
    let x2 = lerp(n001, n101, u);
    let x3 = lerp(n011, n111, u);
    let y0 = lerp(x0, x1, v);
    let y1 = lerp(x2, x3, v);
    lerp(y0, y1, w)
}

fn simplex(pos: [f32; 3], seed: u64) -> f32 {
    const F3: f32 = 1.0 / 3.0;
    const G3: f32 = 1.0 / 6.0;

    let (x, y, z) = (pos[0], pos[1], pos[2]);
    let s = (x + y + z) * F3;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let k = (z + s).floor() as i32;

    let t = (i + j + k) as f32 * G3;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);
    let z0 = z - (k as f32 - t);

    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else if y0 < z0 {
        (0, 0, 1, 0, 1, 1)
    } else if x0 < z0 {
        (0, 1, 0, 0, 1, 1)
    } else {
        (0, 1, 0, 1, 1, 0)
    };

    let x1 = x0 - i1 as f32 + G3;
    let y1 = y0 - j1 as f32 + G3;
    let z1 = z0 - k1 as f32 + G3;
    let x2 = x0 - i2 as f32 + 2.0 * G3;
    let y2 = y0 - j2 as f32 + 2.0 * G3;
    let z2 = z0 - k2 as f32 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    let corner = |cx: f32, cy: f32, cz: f32, gi: i32, gj: i32, gk: i32| -> f32 {
        let t = 0.6 - cx * cx - cy * cy - cz * cz;
        if t < 0.0 {
            0.0
        } else {
            let t2 = t * t;
            let g = GRADIENTS[hash(gi, gj, gk, seed)];
            t2 * t2 * (g[0] * cx + g[1] * cy + g[2] * cz)
        }
    };

    32.0 * (corner(x0, y0, z0, i, j, k)
        + corner(x1, y1, z1, i + i1, j + j1, k + k1)
        + corner(x2, y2, z2, i + i2, j + j2, k + k2)
        + corner(x3, y3, z3, i + 1, j + 1, k + 1))
}

fn cell_feature_point(cx: i32, cy: i32, cz: i32, seed: u64) -> [f32; 3] {
    let hash_xyz = |x: i32, y: i32, z: i32, seed: u64| -> u64 {
        let mut h = (x as u64).wrapping_mul(73_856_093)
            ^ (y as u64).wrapping_mul(19_349_663)
            ^ (z as u64).wrapping_mul(83_492_791)
            ^ seed;
        h = h.wrapping_mul(0x517c_c1b7_2722_0a95);
        h ^= h >> 32;
        h
    };

    let hx = hash_xyz(cx, cy, cz, seed);
    let hy = hash_xyz(cy, cz, cx, seed.wrapping_add(1));
    let hz = hash_xyz(cz, cx, cy, seed.wrapping_add(2));

    [
        (hx & 0xFFFF) as f32 / 65535.0,
        (hy & 0xFFFF) as f32 / 65535.0,
        (hz & 0xFFFF) as f32 / 65535.0,
    ]
}

fn worley_f1_f2(pos: [f32; 3], seed: u64) -> (f32, f32) {
    let xi = pos[0].floor() as i32;
    let yi = pos[1].floor() as i32;
    let zi = pos[2].floor() as i32;

    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;

    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cx = xi + dx;
                let cy = yi + dy;
                let cz = zi + dz;
                let fp = cell_feature_point(cx, cy, cz, seed);
                let px = cx as f32 + fp[0] - pos[0];
                let py = cy as f32 + fp[1] - pos[1];
                let pz = cz as f32 + fp[2] - pos[2];
                let dist = (px * px + py * py + pz * pz).sqrt();
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

fn base_noise(pos: [f32; 3], noise_type: DisplaceNoiseType, seed: u64) -> f32 {
    match noise_type {
        DisplaceNoiseType::Perlin => perlin(pos, seed),
        DisplaceNoiseType::Simplex => simplex(pos, seed),
        DisplaceNoiseType::Worley => {
            let (f1, _) = worley_f1_f2(pos, seed);
            f1.clamp(0.0, 1.0) * 2.0 - 1.0
        }
        DisplaceNoiseType::WorleyF2F1 => {
            let (f1, f2) = worley_f1_f2(pos, seed);
            (f2 - f1).clamp(0.0, 1.0) * 2.0 - 1.0
        }
    }
}

fn eval_noise(pos: [f32; 3], params: &DisplaceNoiseParams) -> f32 {
    match params.fractal {
        DisplaceNoiseFractal::None => base_noise(pos, params.noise_type, params.seed),
        DisplaceNoiseFractal::Standard => {
            let mut result = 0.0;
            let mut amp = 1.0;
            let mut freq = 1.0;
            let mut total = 0.0;
            for _ in 0..params.octaves.max(1) {
                result += amp
                    * base_noise(
                        [pos[0] * freq, pos[1] * freq, pos[2] * freq],
                        params.noise_type,
                        params.seed,
                    );
                total += amp;
                freq *= params.lacunarity;
                amp *= params.roughness;
            }
            if total > 0.0 { result / total } else { result }
        }
        DisplaceNoiseFractal::Terrain => {
            let mut result = 1.0 - base_noise(pos, params.noise_type, params.seed).abs();
            let mut amp = 1.0;
            let mut freq = 1.0;
            let mut weight = result;
            for _ in 1..params.octaves.max(1) {
                freq *= params.lacunarity;
                amp *= params.roughness;
                weight = weight.clamp(0.0, 1.0);
                let signal = (1.0
                    - base_noise(
                        [pos[0] * freq, pos[1] * freq, pos[2] * freq],
                        params.noise_type,
                        params.seed,
                    )
                    .abs())
                    * amp;
                result += weight * signal;
                weight *= signal;
            }
            result.clamp(-1.0, 1.0)
        }
    }
}

fn eval_noise_rgba(sample_coord: [f32; 2], params: &DisplaceNoiseParams) -> [f32; 4] {
    let pos = [
        sample_coord[0] * params.scale[0] + params.offset[0],
        sample_coord[1] * params.scale[1] + params.offset[1],
        params.offset[2],
    ];
    let scalar = ((eval_noise(pos, params) + 1.0) * 0.5).clamp(0.0, 1.0);
    [scalar, scalar, scalar, 1.0]
}

fn newell_sum(positions: &[Vec3]) -> Vec3 {
    let mut nx = 0.0_f32;
    let mut ny = 0.0_f32;
    let mut nz = 0.0_f32;
    for i in 0..positions.len() {
        let cur = positions[i];
        let next = positions[(i + 1) % positions.len()];
        nx += (cur.y - next.y) * (cur.z + next.z);
        ny += (cur.z - next.z) * (cur.x + next.x);
        nz += (cur.x - next.x) * (cur.y + next.y);
    }
    Vec3::new(nx, ny, nz)
}

fn load_uvs(geo: &Geometry, name: &str) -> Result<Option<Vec<[f32; 2]>>, SopError> {
    match geo.attrib_type(AttribClass::Point, name) {
        Some(AttribType::Vector2) => {
            let handle = geo
                .find_attrib::<[f32; 2]>(AttribClass::Point, name)
                .map_err(SopError::Core)?;
            let mut values = Vec::with_capacity(geo.num_points());
            for i in 0..geo.num_points() {
                values.push(geo.get_attrib(&handle, i).map_err(SopError::Core)?);
            }
            return Ok(Some(values));
        }
        Some(AttribType::Vector3) => {
            let handle = geo
                .find_attrib::<[f32; 3]>(AttribClass::Point, name)
                .map_err(SopError::Core)?;
            let mut values = Vec::with_capacity(geo.num_points());
            for i in 0..geo.num_points() {
                let uvw = geo.get_attrib(&handle, i).map_err(SopError::Core)?;
                values.push([uvw[0], uvw[1]]);
            }
            return Ok(Some(values));
        }
        Some(other) => {
            return Err(SopError::InvalidParam(format!(
                "point attribute '{name}' must be Vector2 or Vector3 for UV sampling, got {other:?}"
            )));
        }
        None => {}
    }

    match geo.attrib_type(AttribClass::Vertex, name) {
        Some(AttribType::Vector2) => {
            let handle = geo
                .find_attrib::<[f32; 2]>(AttribClass::Vertex, name)
                .map_err(SopError::Core)?;
            let mut sums = vec![[0.0, 0.0]; geo.num_points()];
            let mut counts = vec![0u32; geo.num_points()];
            for vertex_idx in 0..geo.num_vertices() {
                let point = geo
                    .vertex_point(procgeo_core::VertexHandle::from_index(vertex_idx))
                    .index();
                let uv = geo
                    .get_attrib(&handle, vertex_idx)
                    .map_err(SopError::Core)?;
                sums[point][0] += uv[0];
                sums[point][1] += uv[1];
                counts[point] += 1;
            }

            let mut averaged = vec![[0.0, 0.0]; geo.num_points()];
            for point_idx in 0..geo.num_points() {
                if counts[point_idx] > 0 {
                    let inv = 1.0 / counts[point_idx] as f32;
                    averaged[point_idx] = [sums[point_idx][0] * inv, sums[point_idx][1] * inv];
                }
            }
            return Ok(Some(averaged));
        }
        Some(AttribType::Vector3) => {
            let handle = geo
                .find_attrib::<[f32; 3]>(AttribClass::Vertex, name)
                .map_err(SopError::Core)?;
            let mut sums = vec![[0.0, 0.0]; geo.num_points()];
            let mut counts = vec![0u32; geo.num_points()];
            for vertex_idx in 0..geo.num_vertices() {
                let point = geo
                    .vertex_point(procgeo_core::VertexHandle::from_index(vertex_idx))
                    .index();
                let uvw = geo
                    .get_attrib(&handle, vertex_idx)
                    .map_err(SopError::Core)?;
                sums[point][0] += uvw[0];
                sums[point][1] += uvw[1];
                counts[point] += 1;
            }

            let mut averaged = vec![[0.0, 0.0]; geo.num_points()];
            for point_idx in 0..geo.num_points() {
                if counts[point_idx] > 0 {
                    let inv = 1.0 / counts[point_idx] as f32;
                    averaged[point_idx] = [sums[point_idx][0] * inv, sums[point_idx][1] * inv];
                }
            }
            return Ok(Some(averaged));
        }
        Some(other) => {
            return Err(SopError::InvalidParam(format!(
                "vertex attribute '{name}' must be Vector2 or Vector3 for UV sampling, got {other:?}"
            )));
        }
        None => {}
    }

    Ok(None)
}

fn point_normals(geo: &Geometry, name: &str) -> Result<Vec<Vec3>, SopError> {
    match geo.attrib_type(AttribClass::Point, name) {
        Some(AttribType::Vector3) => {
            let handle = geo
                .find_attrib::<[f32; 3]>(AttribClass::Point, name)
                .map_err(SopError::Core)?;
            let mut normals = Vec::with_capacity(geo.num_points());
            for i in 0..geo.num_points() {
                let n = geo.get_attrib(&handle, i).map_err(SopError::Core)?;
                normals.push(Vec3::new(n[0], n[1], n[2]).normalize_or_zero());
            }
            return Ok(normals);
        }
        Some(other) => {
            return Err(SopError::InvalidParam(format!(
                "point normal attribute '{name}' must be Vector3, got {other:?}"
            )));
        }
        None => {}
    }

    let mut normals = vec![Vec3::ZERO; geo.num_points()];
    for prim_idx in 0..geo.num_prims() {
        let prim_handle = PrimHandle::from_index(prim_idx);
        let prim = geo.prim(prim_handle);
        let is_closed = matches!(
            prim,
            Primitive::Polygon(poly) if poly.poly_type == procgeo_core::PolyType::Closed
        );
        if !is_closed {
            continue;
        }

        let points = geo.prim_points(prim_handle);
        if points.len() < 3 {
            continue;
        }

        let positions: Vec<Vec3> = points.iter().map(|&pt| geo.point_pos(pt)).collect();
        let face_sum = newell_sum(&positions);
        for point in points {
            normals[point.index()] += face_sum;
        }
    }

    for normal in &mut normals {
        *normal = if normal.length_squared() > 1e-10 {
            normal.normalize()
        } else {
            Vec3::Y
        };
    }

    Ok(normals)
}

fn project_position(
    pos: Vec3,
    min: Vec3,
    max: Vec3,
    mode: DisplaceCoordinates,
    projection: DisplaceProjection,
) -> [f32; 2] {
    let select = |value: Vec3| -> [f32; 2] {
        match projection {
            DisplaceProjection::XY => [value.x, value.y],
            DisplaceProjection::XZ => [value.x, value.z],
            DisplaceProjection::YZ => [value.y, value.z],
        }
    };

    match mode {
        DisplaceCoordinates::BoundingBox | DisplaceCoordinates::Auto => {
            let p = select(pos);
            let lo = select(min);
            let hi = select(max);
            [
                if (hi[0] - lo[0]).abs() > 1e-6 {
                    (p[0] - lo[0]) / (hi[0] - lo[0])
                } else {
                    0.5
                },
                if (hi[1] - lo[1]).abs() > 1e-6 {
                    (p[1] - lo[1]) / (hi[1] - lo[1])
                } else {
                    0.5
                },
            ]
        }
        DisplaceCoordinates::Position => select(pos),
        DisplaceCoordinates::UV => [0.0, 0.0],
    }
}

fn sample_channel(rgba: [f32; 4], channel: DisplaceSampleChannel) -> f32 {
    match channel {
        DisplaceSampleChannel::Luminance => rgba[0] * 0.2126 + rgba[1] * 0.7152 + rgba[2] * 0.0722,
        DisplaceSampleChannel::Red => rgba[0],
        DisplaceSampleChannel::Green => rgba[1],
        DisplaceSampleChannel::Blue => rgba[2],
        DisplaceSampleChannel::Alpha => rgba[3],
        DisplaceSampleChannel::Average => (rgba[0] + rgba[1] + rgba[2]) / 3.0,
    }
}

impl Sop for DisplaceSop {
    type Params = DisplaceParams;

    fn name(&self) -> &'static str {
        "displace"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        if geo.num_points() == 0 || params.strength.abs() <= f32::EPSILON {
            return Ok(geo.clone());
        }

        if params.texture.is_some() && params.noise.is_some() {
            return Err(SopError::InvalidParam(
                "displace accepts either a texture or procedural noise, not both".to_string(),
            ));
        }

        if params.texture.is_none() && params.noise.is_none() {
            return Ok(geo.clone());
        }

        if let Some(texture) = &params.texture {
            texture.validate()?;
        }

        let uvs = load_uvs(geo, &params.uv_attrib)?;
        let use_uv = match params.coordinates {
            DisplaceCoordinates::UV => {
                if uvs.is_none() {
                    return Err(SopError::InvalidParam(format!(
                        "UV sampling requested but attribute '{}' was not found on points or vertices",
                        params.uv_attrib
                    )));
                }
                true
            }
            DisplaceCoordinates::Auto => uvs.is_some(),
            _ => false,
        };

        let normals = if params.direction == DisplaceDirection::Normal {
            Some(point_normals(geo, &params.normal_attrib)?)
        } else {
            None
        };

        let bbox = geo.bounding_box();
        let mut out = geo.clone();

        for point_idx in 0..geo.num_points() {
            let point = PointHandle::from_index(point_idx);
            let pos = geo.point_pos(point);

            let mut sample_coord = if use_uv {
                uvs.as_ref().unwrap()[point_idx]
            } else {
                project_position(
                    pos,
                    bbox.min,
                    bbox.max,
                    params.coordinates,
                    params.projection,
                )
            };

            sample_coord[0] = sample_coord[0] * params.coord_scale[0] + params.coord_offset[0];
            sample_coord[1] = sample_coord[1] * params.coord_scale[1] + params.coord_offset[1];

            let rgba = match (&params.texture, &params.noise) {
                (Some(texture), None) => texture.sample(sample_coord, params.sampler, params.wrap),
                (None, Some(noise)) => eval_noise_rgba(sample_coord, noise),
                _ => unreachable!("validated source combination above"),
            };

            let scalar =
                (sample_channel(rgba, params.sample_channel) - params.midlevel) * params.strength;
            let offset = match params.direction {
                DisplaceDirection::Normal => normals.as_ref().unwrap()[point_idx] * scalar,
                DisplaceDirection::X => Vec3::X * scalar,
                DisplaceDirection::Y => Vec3::Y * scalar,
                DisplaceDirection::Z => Vec3::Z * scalar,
                DisplaceDirection::CustomVector => {
                    let axis = Vec3::new(
                        params.custom_vector[0],
                        params.custom_vector[1],
                        params.custom_vector[2],
                    )
                    .normalize_or_zero();
                    axis * scalar
                }
                DisplaceDirection::RGBToXYZ => {
                    Vec3::new(
                        rgba[0] - params.midlevel,
                        rgba[1] - params.midlevel,
                        rgba[2] - params.midlevel,
                    ) * params.strength
                }
            };

            out.set_point_pos(point, pos + offset);
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GeometryExt;
    use approx::assert_relative_eq;
    use procgeo_core::{AttribDefault, TypeQualifier};

    fn make_quad() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(-0.5, 0.0, -0.5));
        let p1 = geo.add_point(Vec3::new(0.5, 0.0, -0.5));
        let p2 = geo.add_point(Vec3::new(0.5, 0.0, 0.5));
        let p3 = geo.add_point(Vec3::new(-0.5, 0.0, 0.5));
        geo.add_face(&[p0, p3, p2, p1]);
        geo
    }

    fn stripe_texture() -> DisplaceTexture {
        DisplaceTexture {
            width: 2,
            height: 1,
            pixels: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn displace_default_is_noop_without_source() {
        let quad = make_quad();
        let result = quad
            .clone()
            .apply(&DisplaceSop, &DisplaceParams::default())
            .unwrap();

        for i in 0..quad.num_points() {
            let before = quad.point_pos(PointHandle::from_index(i));
            let after = result.point_pos(PointHandle::from_index(i));
            assert_relative_eq!(before.x, after.x, epsilon = 1e-6);
            assert_relative_eq!(before.y, after.y, epsilon = 1e-6);
            assert_relative_eq!(before.z, after.z, epsilon = 1e-6);
        }
    }

    #[test]
    fn displace_texture_uses_bbox_projection() {
        let quad = make_quad();
        let result = quad
            .apply(
                &DisplaceSop,
                &DisplaceParams {
                    texture: Some(stripe_texture()),
                    direction: DisplaceDirection::Y,
                    coordinates: DisplaceCoordinates::BoundingBox,
                    projection: DisplaceProjection::XZ,
                    sampler: DisplaceSampler::Nearest,
                    wrap: DisplaceWrapMode::Clamp,
                    midlevel: 0.0,
                    ..Default::default()
                },
            )
            .unwrap();

        assert_relative_eq!(
            result.point_pos(PointHandle::from_index(0)).y,
            0.0,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            result.point_pos(PointHandle::from_index(1)).y,
            1.0,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            result.point_pos(PointHandle::from_index(2)).y,
            1.0,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            result.point_pos(PointHandle::from_index(3)).y,
            0.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn displace_texture_uses_uvs_for_normal_direction() {
        let mut quad = make_quad();
        quad.add_attrib(
            AttribClass::Point,
            "uv",
            AttribDefault::Vector2([0.0, 0.0]),
            TypeQualifier::Vector,
        )
        .unwrap();
        let handle = quad
            .find_attrib::<[f32; 2]>(AttribClass::Point, "uv")
            .unwrap();
        quad.set_attrib(&handle, 0, [0.0, 0.0]).unwrap();
        quad.set_attrib(&handle, 1, [1.0, 0.0]).unwrap();
        quad.set_attrib(&handle, 2, [1.0, 1.0]).unwrap();
        quad.set_attrib(&handle, 3, [0.0, 1.0]).unwrap();

        let result = quad
            .apply(
                &DisplaceSop,
                &DisplaceParams {
                    texture: Some(stripe_texture()),
                    coordinates: DisplaceCoordinates::UV,
                    sampler: DisplaceSampler::Nearest,
                    wrap: DisplaceWrapMode::Clamp,
                    midlevel: 0.0,
                    ..Default::default()
                },
            )
            .unwrap();

        assert_relative_eq!(
            result.point_pos(PointHandle::from_index(0)).y,
            0.0,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            result.point_pos(PointHandle::from_index(1)).y,
            1.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn displace_rgb_to_xyz_uses_color_channels() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::ZERO);

        let result = geo
            .apply(
                &DisplaceSop,
                &DisplaceParams {
                    texture: Some(DisplaceTexture {
                        width: 1,
                        height: 1,
                        pixels: vec![1.0, 0.5, 0.0, 1.0],
                    }),
                    direction: DisplaceDirection::RGBToXYZ,
                    coordinates: DisplaceCoordinates::Position,
                    projection: DisplaceProjection::XY,
                    midlevel: 0.5,
                    ..Default::default()
                },
            )
            .unwrap();

        let pos = result.point_pos(PointHandle::from_index(0));
        assert_relative_eq!(pos.x, 0.5, epsilon = 1e-6);
        assert_relative_eq!(pos.y, 0.0, epsilon = 1e-6);
        assert_relative_eq!(pos.z, -0.5, epsilon = 1e-6);
    }

    #[test]
    fn displace_noise_is_deterministic() {
        let quad = make_quad();
        let params = DisplaceParams {
            direction: DisplaceDirection::Y,
            coordinates: DisplaceCoordinates::BoundingBox,
            projection: DisplaceProjection::XZ,
            midlevel: 0.5,
            noise: Some(DisplaceNoiseParams {
                seed: 42,
                fractal: DisplaceNoiseFractal::Standard,
                ..Default::default()
            }),
            ..Default::default()
        };

        let a = quad.clone().apply(&DisplaceSop, &params).unwrap();
        let b = quad.apply(&DisplaceSop, &params).unwrap();

        for i in 0..a.num_points() {
            let pa = a.point_pos(PointHandle::from_index(i));
            let pb = b.point_pos(PointHandle::from_index(i));
            assert_relative_eq!(pa.y, pb.y, epsilon = 1e-6);
        }
    }

    #[test]
    fn displace_rejects_invalid_texture_payload() {
        let quad = make_quad();
        let err = quad
            .apply(
                &DisplaceSop,
                &DisplaceParams {
                    texture: Some(DisplaceTexture {
                        width: 2,
                        height: 2,
                        pixels: vec![0.0; 4],
                    }),
                    ..Default::default()
                },
            )
            .unwrap_err();

        assert!(err.to_string().contains("texture expects 16 RGBA floats"));
    }

    #[test]
    fn displace_rejects_multiple_sources() {
        let quad = make_quad();
        let err = quad
            .apply(
                &DisplaceSop,
                &DisplaceParams {
                    texture: Some(stripe_texture()),
                    noise: Some(DisplaceNoiseParams::default()),
                    ..Default::default()
                },
            )
            .unwrap_err();

        assert!(
            err.to_string()
                .contains("either a texture or procedural noise")
        );
    }
}
