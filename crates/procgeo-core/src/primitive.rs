use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::handle::{PrimHandle, VertexHandle};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PolyType {
    Open,
    Closed,
}

#[derive(Clone, Debug)]
pub struct PolygonPrim {
    pub vertices: SmallVec<[VertexHandle; 4]>,
    pub poly_type: PolyType,
}

impl PolygonPrim {
    pub fn new(vertices: SmallVec<[VertexHandle; 4]>, poly_type: PolyType) -> Self {
        Self {
            vertices,
            poly_type,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Primitive {
    Polygon(PolygonPrim),
}

impl Primitive {
    pub fn vertices(&self) -> &[VertexHandle] {
        match self {
            Primitive::Polygon(p) => &p.vertices,
        }
    }

    pub fn vertex_count(&self) -> usize {
        match self {
            Primitive::Polygon(p) => p.vertices.len(),
        }
    }
}

/// Storage for all primitives in a geometry.
#[derive(Clone)]
pub struct PrimStorage {
    prims: Vec<Primitive>,
}

impl PrimStorage {
    pub fn new() -> Self {
        Self { prims: Vec::new() }
    }

    pub fn add(&mut self, prim: Primitive) -> PrimHandle {
        let idx = self.prims.len();
        self.prims.push(prim);
        PrimHandle::from_index(idx)
    }

    pub fn get(&self, handle: PrimHandle) -> &Primitive {
        &self.prims[handle.index()]
    }

    pub fn get_mut(&mut self, handle: PrimHandle) -> &mut Primitive {
        &mut self.prims[handle.index()]
    }

    pub fn len(&self) -> usize {
        self.prims.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prims.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Primitive> {
        self.prims.iter()
    }

    pub fn clear(&mut self) {
        self.prims.clear();
    }
}

impl Default for PrimStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_polygon() {
        let mut storage = PrimStorage::new();
        let verts: SmallVec<[VertexHandle; 4]> =
            (0..3).map(|i| VertexHandle::from_index(i)).collect();
        let prim = Primitive::Polygon(PolygonPrim::new(verts, PolyType::Closed));
        let handle = storage.add(prim);

        assert_eq!(storage.len(), 1);
        assert_eq!(storage.get(handle).vertex_count(), 3);
    }

    #[test]
    fn polygon_vertices() {
        let mut storage = PrimStorage::new();
        let verts: SmallVec<[VertexHandle; 4]> =
            (0..4).map(|i| VertexHandle::from_index(i)).collect();
        let prim = Primitive::Polygon(PolygonPrim::new(verts, PolyType::Closed));
        let handle = storage.add(prim);

        let vertices = storage.get(handle).vertices();
        assert_eq!(vertices.len(), 4);
        assert_eq!(vertices[0], VertexHandle::from_index(0));
        assert_eq!(vertices[3], VertexHandle::from_index(3));
    }

    #[test]
    fn smallvec_inline_for_quad() {
        let verts: SmallVec<[VertexHandle; 4]> =
            (0..4).map(|i| VertexHandle::from_index(i)).collect();
        // A quad (4 vertices) should fit inline without heap allocation.
        assert!(!verts.spilled());
    }
}
