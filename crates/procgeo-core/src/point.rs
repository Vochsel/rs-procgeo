use glam::Vec3;

use crate::handle::PointHandle;

/// SoA (Structure of Arrays) point storage with contiguous x/y/z layout.
#[derive(Clone)]
pub struct PointStorage {
    x: Vec<f32>,
    y: Vec<f32>,
    z: Vec<f32>,
}

impl PointStorage {
    pub fn new() -> Self {
        Self {
            x: Vec::new(),
            y: Vec::new(),
            z: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            x: Vec::with_capacity(cap),
            y: Vec::with_capacity(cap),
            z: Vec::with_capacity(cap),
        }
    }

    pub fn add(&mut self, pos: Vec3) -> PointHandle {
        let idx = self.x.len();
        self.x.push(pos.x);
        self.y.push(pos.y);
        self.z.push(pos.z);
        PointHandle::from_index(idx)
    }

    pub fn position(&self, handle: PointHandle) -> Vec3 {
        let i = handle.index();
        Vec3::new(self.x[i], self.y[i], self.z[i])
    }

    pub fn set_position(&mut self, handle: PointHandle, pos: Vec3) {
        let i = handle.index();
        self.x[i] = pos.x;
        self.y[i] = pos.y;
        self.z[i] = pos.z;
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.x
            .iter()
            .zip(self.y.iter())
            .zip(self.z.iter())
            .map(|((x, y), z)| Vec3::new(*x, *y, *z))
    }

    pub fn x_slice(&self) -> &[f32] {
        &self.x
    }

    pub fn y_slice(&self) -> &[f32] {
        &self.y
    }

    pub fn z_slice(&self) -> &[f32] {
        &self.z
    }

    pub fn reserve(&mut self, additional: usize) {
        self.x.reserve(additional);
        self.y.reserve(additional);
        self.z.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.x.clear();
        self.y.clear();
        self.z.clear();
    }
}

impl Default for PointStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn add_and_get() {
        let mut storage = PointStorage::new();
        let h = storage.add(Vec3::new(1.0, 2.0, 3.0));
        let pos = storage.position(h);
        assert_relative_eq!(pos.x, 1.0);
        assert_relative_eq!(pos.y, 2.0);
        assert_relative_eq!(pos.z, 3.0);
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn set_position() {
        let mut storage = PointStorage::new();
        let h = storage.add(Vec3::new(0.0, 0.0, 0.0));
        storage.set_position(h, Vec3::new(5.0, 6.0, 7.0));
        let pos = storage.position(h);
        assert_relative_eq!(pos.x, 5.0);
        assert_relative_eq!(pos.y, 6.0);
        assert_relative_eq!(pos.z, 7.0);
    }

    #[test]
    fn soa_layout() {
        let mut storage = PointStorage::new();
        storage.add(Vec3::new(1.0, 2.0, 3.0));
        storage.add(Vec3::new(4.0, 5.0, 6.0));
        storage.add(Vec3::new(7.0, 8.0, 9.0));

        assert_eq!(storage.x_slice(), &[1.0, 4.0, 7.0]);
        assert_eq!(storage.y_slice(), &[2.0, 5.0, 8.0]);
        assert_eq!(storage.z_slice(), &[3.0, 6.0, 9.0]);
    }

    #[test]
    fn iter() {
        let mut storage = PointStorage::new();
        storage.add(Vec3::new(1.0, 0.0, 0.0));
        storage.add(Vec3::new(0.0, 1.0, 0.0));

        let positions: Vec<Vec3> = storage.iter().collect();
        assert_eq!(positions.len(), 2);
        assert_relative_eq!(positions[0].x, 1.0);
        assert_relative_eq!(positions[1].y, 1.0);
    }

    #[test]
    fn with_capacity() {
        let storage = PointStorage::with_capacity(64);
        assert!(storage.is_empty());
        assert_eq!(storage.len(), 0);
    }
}
