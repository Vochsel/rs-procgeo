use glam::Vec3;

/// Axis-aligned bounding box.
#[derive(Clone, Debug)]
pub struct BBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BBox {
    /// An inverted (empty) bounding box. Expanding with any point yields a
    /// valid box containing exactly that point.
    pub fn empty() -> Self {
        BBox {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    pub fn new(min: Vec3, max: Vec3) -> Self {
        BBox { min, max }
    }

    /// Expand to include the given point.
    pub fn expand_point(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    /// Expand to include another BBox.
    pub fn expand_bbox(&mut self, other: &BBox) {
        if other.is_valid() {
            self.expand_point(other.min);
            self.expand_point(other.max);
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn contains(&self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    pub fn intersects(&self, other: &BBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Returns true when min <= max on all axes (non-empty, non-inverted).
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    /// Build a BBox directly from SoA point slices (no Vec3 intermediate).
    pub fn from_soa(x: &[f32], y: &[f32], z: &[f32]) -> BBox {
        let mut bbox = BBox::empty();
        for i in 0..x.len().min(y.len()).min(z.len()) {
            bbox.expand_point(Vec3::new(x[i], y[i], z[i]));
        }
        bbox
    }
}

impl Default for BBox {
    fn default() -> Self {
        BBox::empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn bbox_empty() {
        let b = BBox::empty();
        assert!(!b.is_valid());
    }

    #[test]
    fn bbox_expand_point() {
        let mut b = BBox::empty();
        b.expand_point(Vec3::new(1.0, 2.0, 3.0));
        b.expand_point(Vec3::new(-1.0, 0.0, 5.0));

        assert!(b.is_valid());
        assert_relative_eq!(b.min.x, -1.0);
        assert_relative_eq!(b.min.y, 0.0);
        assert_relative_eq!(b.min.z, 3.0);
        assert_relative_eq!(b.max.x, 1.0);
        assert_relative_eq!(b.max.y, 2.0);
        assert_relative_eq!(b.max.z, 5.0);
    }

    #[test]
    fn bbox_center_and_size() {
        let b = BBox::new(Vec3::ZERO, Vec3::new(2.0, 4.0, 6.0));
        let c = b.center();
        assert_relative_eq!(c.x, 1.0);
        assert_relative_eq!(c.y, 2.0);
        assert_relative_eq!(c.z, 3.0);

        let s = b.size();
        assert_relative_eq!(s.x, 2.0);
        assert_relative_eq!(s.y, 4.0);
        assert_relative_eq!(s.z, 6.0);
    }

    #[test]
    fn bbox_contains() {
        let b = BBox::new(Vec3::ZERO, Vec3::ONE);
        assert!(b.contains(Vec3::new(0.5, 0.5, 0.5)));
        assert!(b.contains(Vec3::ZERO));
        assert!(b.contains(Vec3::ONE));
        assert!(!b.contains(Vec3::new(1.1, 0.5, 0.5)));
    }

    #[test]
    fn bbox_intersects() {
        let a = BBox::new(Vec3::ZERO, Vec3::ONE);
        let b = BBox::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(1.5, 1.5, 1.5));
        let c = BBox::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));

        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn bbox_from_soa() {
        let x = [0.0f32, 1.0, -1.0];
        let y = [0.0f32, 2.0, -2.0];
        let z = [0.0f32, 3.0, -3.0];

        let b = BBox::from_soa(&x, &y, &z);
        assert!(b.is_valid());
        assert_relative_eq!(b.min.x, -1.0);
        assert_relative_eq!(b.min.y, -2.0);
        assert_relative_eq!(b.min.z, -3.0);
        assert_relative_eq!(b.max.x, 1.0);
        assert_relative_eq!(b.max.y, 2.0);
        assert_relative_eq!(b.max.z, 3.0);
    }
}
