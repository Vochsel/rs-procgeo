// Inside/outside classification for Boolean SOP using generalized winding number.
//
// Uses the Van Oosterom and Strackee solid-angle formula to compute the
// generalized winding number of a point with respect to a triangle mesh.
// A winding number of ~1 indicates the point is inside a closed mesh,
// ~0 indicates outside.

use glam::Vec3;

use super::bvh::Triangle;

/// Returns `true` if `point` is inside the mesh (|winding_number| > 0.5).
pub fn is_inside_mesh(point: Vec3, triangles: &[Triangle]) -> bool {
    generalized_winding_number(point, triangles).abs() > 0.5
}

/// Compute the generalized winding number of `point` with respect to the
/// triangle mesh. For a closed mesh with outward-facing normals the value is
/// approximately 1 inside and 0 outside.
///
/// The computation is performed in f64 to avoid catastrophic cancellation in
/// the solid-angle formula when the point is far from a triangle.
pub fn generalized_winding_number(point: Vec3, triangles: &[Triangle]) -> f32 {
    let p = dvec3(point);
    let mut total: f64 = 0.0;

    for tri in triangles {
        total += solid_angle(p, dvec3(tri.v0), dvec3(tri.v1), dvec3(tri.v2));
    }

    (total / (4.0 * std::f64::consts::PI)) as f32
}

/// Classify the winding-number depth of `point` (round to the nearest integer).
/// For a single closed mesh the result is 1 inside and 0 outside; nested shells
/// can produce higher values.
pub fn classify_depth(point: Vec3, triangles: &[Triangle]) -> i32 {
    generalized_winding_number(point, triangles).round() as i32
}

// ---------------------------------------------------------------------------
// Internal helpers (f64 precision)
// ---------------------------------------------------------------------------

/// Tiny 3-component f64 vector — just enough for the solid-angle formula.
#[derive(Debug, Clone, Copy)]
struct DVec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl DVec3 {
    fn dot(self, other: DVec3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: DVec3) -> DVec3 {
        DVec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    fn sub(self, other: DVec3) -> DVec3 {
        DVec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

/// Promote a glam `Vec3` to our f64 helper.
#[inline]
fn dvec3(v: Vec3) -> DVec3 {
    DVec3 {
        x: v.x as f64,
        y: v.y as f64,
        z: v.z as f64,
    }
}

/// Compute the signed solid angle subtended by triangle (a, b, c) as seen from
/// point p, using the Van Oosterom and Strackee formula.
///
/// solid_angle = 2 * atan2(numerator, denominator)
///
/// where:
///   pa = a - p, pb = b - p, pc = c - p
///   la = |pa|, lb = |pb|, lc = |pc|
///   numerator   = pa . (pb x pc)
///   denominator = la*lb*lc + (pa.pb)*lc + (pa.pc)*lb + (pb.pc)*la
fn solid_angle(p: DVec3, a: DVec3, b: DVec3, c: DVec3) -> f64 {
    let pa = a.sub(p);
    let pb = b.sub(p);
    let pc = c.sub(p);

    let la = pa.length();
    let lb = pb.length();
    let lc = pc.length();

    // Degenerate: point coincides with a triangle vertex.
    if la < 1e-15 || lb < 1e-15 || lc < 1e-15 {
        return 0.0;
    }

    let numerator = pa.dot(pb.cross(pc));
    let denominator =
        la * lb * lc + pa.dot(pb) * lc + pa.dot(pc) * lb + pb.dot(pc) * la;

    2.0 * numerator.atan2(denominator)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    /// Build a unit cube centred at the origin as 12 triangles (6 faces x 2 tris)
    /// with outward-facing normals.
    fn unit_cube_triangles() -> Vec<Triangle> {
        let mut tris = Vec::with_capacity(12);
        let mut idx: usize = 0;

        // Helper: push two CCW triangles for a quad given four corners in CCW
        // order when viewed from the outside.
        let mut push_quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
            tris.push(Triangle { v0: a, v1: b, v2: c, index: idx });
            idx += 1;
            tris.push(Triangle { v0: a, v1: c, v2: d, index: idx });
            idx += 1;
        };

        // Half-extent
        let h = 0.5;

        // +Z face (front)
        push_quad(
            Vec3::new(-h, -h, h),
            Vec3::new(h, -h, h),
            Vec3::new(h, h, h),
            Vec3::new(-h, h, h),
        );
        // -Z face (back)
        push_quad(
            Vec3::new(h, -h, -h),
            Vec3::new(-h, -h, -h),
            Vec3::new(-h, h, -h),
            Vec3::new(h, h, -h),
        );
        // +X face (right)
        push_quad(
            Vec3::new(h, -h, h),
            Vec3::new(h, -h, -h),
            Vec3::new(h, h, -h),
            Vec3::new(h, h, h),
        );
        // -X face (left)
        push_quad(
            Vec3::new(-h, -h, -h),
            Vec3::new(-h, -h, h),
            Vec3::new(-h, h, h),
            Vec3::new(-h, h, -h),
        );
        // +Y face (top)
        push_quad(
            Vec3::new(-h, h, h),
            Vec3::new(h, h, h),
            Vec3::new(h, h, -h),
            Vec3::new(-h, h, -h),
        );
        // -Y face (bottom)
        push_quad(
            Vec3::new(-h, -h, -h),
            Vec3::new(h, -h, -h),
            Vec3::new(h, -h, h),
            Vec3::new(-h, -h, h),
        );

        tris
    }

    #[test]
    fn point_inside_cube() {
        let tris = unit_cube_triangles();
        assert!(
            is_inside_mesh(Vec3::ZERO, &tris),
            "origin should be inside the unit cube"
        );
    }

    #[test]
    fn point_outside_cube() {
        let tris = unit_cube_triangles();
        assert!(
            !is_inside_mesh(Vec3::new(5.0, 0.0, 0.0), &tris),
            "point at (5,0,0) should be outside the unit cube"
        );
    }

    #[test]
    fn depth_inside_is_one() {
        let tris = unit_cube_triangles();
        let depth = classify_depth(Vec3::ZERO, &tris);
        assert_eq!(depth, 1, "depth at centre of closed cube should be 1, got {depth}");
    }

    #[test]
    fn depth_outside_is_zero() {
        let tris = unit_cube_triangles();
        let depth = classify_depth(Vec3::new(5.0, 0.0, 0.0), &tris);
        assert_eq!(depth, 0, "depth far outside the cube should be 0, got {depth}");
    }
}
