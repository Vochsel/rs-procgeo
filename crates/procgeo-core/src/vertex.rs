use crate::handle::{PrimHandle, PointHandle, VertexHandle};

/// Stores vertex topology: each vertex references one point and one primitive.
#[derive(Clone)]
pub struct VertexStorage {
    point_refs: Vec<PointHandle>,
    prim_refs: Vec<PrimHandle>,
}

impl VertexStorage {
    pub fn new() -> Self {
        Self {
            point_refs: Vec::new(),
            prim_refs: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            point_refs: Vec::with_capacity(cap),
            prim_refs: Vec::with_capacity(cap),
        }
    }

    pub fn add(&mut self, point: PointHandle, prim: PrimHandle) -> VertexHandle {
        let idx = self.point_refs.len();
        self.point_refs.push(point);
        self.prim_refs.push(prim);
        VertexHandle::from_index(idx)
    }

    pub fn point(&self, handle: VertexHandle) -> PointHandle {
        self.point_refs[handle.index()]
    }

    pub fn prim(&self, handle: VertexHandle) -> PrimHandle {
        self.prim_refs[handle.index()]
    }

    pub fn len(&self) -> usize {
        self.point_refs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.point_refs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (VertexHandle, PointHandle, PrimHandle)> + '_ {
        self.point_refs
            .iter()
            .zip(self.prim_refs.iter())
            .enumerate()
            .map(|(i, (pt, pr))| (VertexHandle::from_index(i), *pt, *pr))
    }

    pub fn reserve(&mut self, additional: usize) {
        self.point_refs.reserve(additional);
        self.prim_refs.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.point_refs.clear();
        self.prim_refs.clear();
    }
}

impl Default for VertexStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_query() {
        let mut storage = VertexStorage::new();
        let pt = PointHandle::from_index(0);
        let pr = PrimHandle::from_index(0);
        let vh = storage.add(pt, pr);

        assert_eq!(storage.point(vh), pt);
        assert_eq!(storage.prim(vh), pr);
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn multiple_vertices_same_point() {
        let mut storage = VertexStorage::new();
        let pt = PointHandle::from_index(5);
        let pr0 = PrimHandle::from_index(0);
        let pr1 = PrimHandle::from_index(1);

        let v0 = storage.add(pt, pr0);
        let v1 = storage.add(pt, pr1);

        assert_eq!(storage.point(v0), pt);
        assert_eq!(storage.point(v1), pt);
        assert_eq!(storage.prim(v0), pr0);
        assert_eq!(storage.prim(v1), pr1);
        assert_eq!(storage.len(), 2);
    }
}
