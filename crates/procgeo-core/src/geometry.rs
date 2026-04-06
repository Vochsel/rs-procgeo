use std::fmt;

use glam::Vec3;

use crate::attribute::{AttribClass, AttribDefault, AttribHandle, AttribValue, AttributeMap, TypeQualifier};
use crate::error::CoreError;
use crate::group::GroupMap;
use crate::handle::{PrimHandle, PointHandle, VertexHandle};
use crate::math::BBox;
use crate::point::PointStorage;
use crate::primitive::{PolyType, PolygonPrim, PrimStorage, Primitive};
use crate::vertex::VertexStorage;

use smallvec::SmallVec;

/// Central geometry struct tying together points, vertices, primitives,
/// attributes, and groups.
#[derive(Clone)]
pub struct Geometry {
    pub(crate) points: PointStorage,
    pub(crate) vertices: VertexStorage,
    pub(crate) primitives: PrimStorage,
    pub(crate) attributes: AttributeMap,
    pub(crate) groups: GroupMap,
}

impl Geometry {
    pub fn new() -> Self {
        Geometry {
            points: PointStorage::new(),
            vertices: VertexStorage::new(),
            primitives: PrimStorage::new(),
            attributes: AttributeMap::new(),
            groups: GroupMap::new(),
        }
    }

    pub fn with_capacity(points: usize, prims: usize) -> Self {
        Geometry {
            points: PointStorage::with_capacity(points),
            vertices: VertexStorage::with_capacity(prims * 4),
            primitives: PrimStorage::new(),
            attributes: AttributeMap::new(),
            groups: GroupMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Points
    // -----------------------------------------------------------------------

    /// Add a point at `pos`. Also resizes point attributes and point groups.
    pub fn add_point(&mut self, pos: Vec3) -> PointHandle {
        let handle = self.points.add(pos);
        let new_len = self.points.len();
        self.attributes.resize_class(AttribClass::Point, new_len);
        self.groups.resize_point_groups(new_len);
        handle
    }

    pub fn point_pos(&self, handle: PointHandle) -> Vec3 {
        self.points.position(handle)
    }

    pub fn set_point_pos(&mut self, handle: PointHandle, pos: Vec3) {
        self.points.set_position(handle, pos);
    }

    pub fn num_points(&self) -> usize {
        self.points.len()
    }

    pub fn points(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.points.iter()
    }

    pub fn point_storage(&self) -> &PointStorage {
        &self.points
    }

    pub fn point_storage_mut(&mut self) -> &mut PointStorage {
        &mut self.points
    }

    // -----------------------------------------------------------------------
    // Vertices
    // -----------------------------------------------------------------------

    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    pub fn vertex_point(&self, handle: VertexHandle) -> PointHandle {
        self.vertices.point(handle)
    }

    pub fn vertex_prim(&self, handle: VertexHandle) -> PrimHandle {
        self.vertices.prim(handle)
    }

    // -----------------------------------------------------------------------
    // Primitives
    // -----------------------------------------------------------------------

    /// Add a polygon with the given point handles and poly type.
    ///
    /// Creates one vertex per point, then a Polygon primitive referencing them.
    /// Resizes vertex and prim attributes/groups accordingly.
    pub fn add_polygon(
        &mut self,
        point_handles: &[PointHandle],
        poly_type: PolyType,
    ) -> PrimHandle {
        // We need the prim handle before creating vertices, but PrimStorage
        // requires the Primitive to be complete. We stage the prim index first.
        let prim_idx = self.primitives.len();
        let prim_handle = PrimHandle::from_index(prim_idx);

        // Create vertices
        let mut vert_handles: SmallVec<[VertexHandle; 4]> =
            SmallVec::with_capacity(point_handles.len());
        for &pt in point_handles {
            let vh = self.vertices.add(pt, prim_handle);
            vert_handles.push(vh);
        }

        // Create the primitive
        let prim = Primitive::Polygon(PolygonPrim::new(vert_handles, poly_type));
        let returned_handle = self.primitives.add(prim);
        debug_assert_eq!(returned_handle, prim_handle);

        // Resize attribute storage and groups
        let num_verts = self.vertices.len();
        let num_prims = self.primitives.len();
        self.attributes.resize_class(AttribClass::Vertex, num_verts);
        self.attributes
            .resize_class(AttribClass::Primitive, num_prims);
        self.groups.resize_vertex_groups(num_verts);
        self.groups.resize_prim_groups(num_prims);

        prim_handle
    }

    /// Convenience: add a closed face (polygon).
    pub fn add_face(&mut self, point_handles: &[PointHandle]) -> PrimHandle {
        self.add_polygon(point_handles, PolyType::Closed)
    }

    /// Convenience: add an open polyline.
    pub fn add_polyline(&mut self, point_handles: &[PointHandle]) -> PrimHandle {
        self.add_polygon(point_handles, PolyType::Open)
    }

    pub fn num_prims(&self) -> usize {
        self.primitives.len()
    }

    pub fn prims(&self) -> impl Iterator<Item = &Primitive> {
        self.primitives.iter()
    }

    pub fn prim(&self, handle: PrimHandle) -> &Primitive {
        self.primitives.get(handle)
    }

    pub fn prim_vertices(&self, handle: PrimHandle) -> &[VertexHandle] {
        self.primitives.get(handle).vertices()
    }

    pub fn prim_points(&self, handle: PrimHandle) -> Vec<PointHandle> {
        self.primitives
            .get(handle)
            .vertices()
            .iter()
            .map(|&vh| self.vertices.point(vh))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Attributes
    // -----------------------------------------------------------------------

    pub fn add_attrib(
        &mut self,
        class: AttribClass,
        name: impl Into<String>,
        default: AttribDefault,
        qualifier: TypeQualifier,
    ) -> Result<(), CoreError> {
        let name_str: String = name.into();
        self.attributes.create(class, name_str.clone(), default, qualifier)?;
        // Resize to match current element counts
        let count = match class {
            AttribClass::Point => self.points.len(),
            AttribClass::Vertex => self.vertices.len(),
            AttribClass::Primitive => self.primitives.len(),
            AttribClass::Detail => 1,
        };
        self.attributes.resize_class(class, count);
        Ok(())
    }

    pub fn find_attrib<T: AttribValue>(
        &self,
        class: AttribClass,
        name: impl AsRef<str>,
    ) -> Result<AttribHandle<T>, CoreError> {
        self.attributes.find::<T>(class, name)
    }

    pub fn get_attrib<T: AttribValue>(
        &self,
        handle: &AttribHandle<T>,
        index: usize,
    ) -> Result<T, CoreError> {
        self.attributes.get(handle, index)
    }

    pub fn set_attrib<T: AttribValue>(
        &mut self,
        handle: &AttribHandle<T>,
        index: usize,
        value: T,
    ) -> Result<(), CoreError> {
        self.attributes.set(handle, index, value)
    }

    pub fn attributes(&self) -> &AttributeMap {
        &self.attributes
    }

    pub fn attributes_mut(&mut self) -> &mut AttributeMap {
        &mut self.attributes
    }

    // -----------------------------------------------------------------------
    // Groups
    // -----------------------------------------------------------------------

    pub fn groups(&self) -> &GroupMap {
        &self.groups
    }

    pub fn groups_mut(&mut self) -> &mut GroupMap {
        &mut self.groups
    }

    pub fn create_point_group(&mut self, name: impl Into<String>) {
        let size = self.points.len();
        self.groups.create_point_group(name, size);
    }

    pub fn create_prim_group(&mut self, name: impl Into<String>) {
        let size = self.primitives.len();
        self.groups.create_prim_group(name, size);
    }

    // -----------------------------------------------------------------------
    // Rebuild helpers
    // -----------------------------------------------------------------------

    /// Rebuild geometry keeping only primitives where `keep[i]` is true.
    /// Points not referenced by any kept primitive are removed.
    /// Returns a new Geometry with compacted indices.
    pub fn rebuild_keeping_prims(&self, keep: &[bool]) -> Geometry {
        // Determine which points are referenced by kept prims
        let mut point_used = vec![false; self.points.len()];
        for (i, &k) in keep.iter().enumerate() {
            if k && i < self.primitives.len() {
                let ph = PrimHandle::from_index(i);
                for pt in self.prim_points(ph) {
                    point_used[pt.index()] = true;
                }
            }
        }

        // Build point remap: old index -> new index (None if removed)
        let mut point_remap: Vec<Option<usize>> = vec![None; self.points.len()];
        let mut new_geo = Geometry::new();
        for (old_idx, &used) in point_used.iter().enumerate() {
            if used {
                let new_idx = new_geo.num_points();
                point_remap[old_idx] = Some(new_idx);
                let pos = self.points.position(PointHandle::from_index(old_idx));
                new_geo.add_point(pos);
            }
        }

        // Add kept prims with remapped point indices
        for (i, &k) in keep.iter().enumerate() {
            if k && i < self.primitives.len() {
                let ph = PrimHandle::from_index(i);
                let old_pts = self.prim_points(ph);
                let new_pts: Vec<PointHandle> = old_pts
                    .iter()
                    .filter_map(|pt| point_remap[pt.index()].map(PointHandle::from_index))
                    .collect();
                if new_pts.len() == old_pts.len() {
                    let prim = self.primitives.get(ph);
                    match prim {
                        Primitive::Polygon(poly) => match poly.poly_type {
                            PolyType::Closed => { new_geo.add_face(&new_pts); }
                            PolyType::Open => { new_geo.add_polyline(&new_pts); }
                        },
                    }
                }
            }
        }

        new_geo
    }

    /// Rebuild geometry keeping only points where `keep[i]` is true.
    /// Primitives referencing any removed point are also removed.
    pub fn rebuild_keeping_points(&self, keep: &[bool]) -> Geometry {
        // Build point remap: old index -> new index (None if removed)
        let mut point_remap: Vec<Option<usize>> = vec![None; self.points.len()];
        let mut new_geo = Geometry::new();
        for (old_idx, &kept) in keep.iter().enumerate() {
            if kept {
                let new_idx = new_geo.num_points();
                point_remap[old_idx] = Some(new_idx);
                let pos = self.points.position(PointHandle::from_index(old_idx));
                new_geo.add_point(pos);
            }
        }

        // Add prims whose all points are kept
        for i in 0..self.primitives.len() {
            let ph = PrimHandle::from_index(i);
            let old_pts = self.prim_points(ph);
            let all_kept = old_pts.iter().all(|pt| {
                pt.index() < keep.len() && keep[pt.index()]
            });
            if all_kept {
                let new_pts: Vec<PointHandle> = old_pts
                    .iter()
                    .filter_map(|pt| point_remap[pt.index()].map(PointHandle::from_index))
                    .collect();
                let prim = self.primitives.get(ph);
                match prim {
                    Primitive::Polygon(poly) => match poly.poly_type {
                        PolyType::Closed => { new_geo.add_face(&new_pts); }
                        PolyType::Open => { new_geo.add_polyline(&new_pts); }
                    },
                }
            }
        }

        new_geo
    }

    // -----------------------------------------------------------------------
    // Spatial
    // -----------------------------------------------------------------------

    pub fn bounding_box(&self) -> BBox {
        BBox::from_soa(
            self.points.x_slice(),
            self.points.y_slice(),
            self.points.z_slice(),
        )
    }
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Geometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Geometry")
            .field("points", &self.points.len())
            .field("vertices", &self.vertices.len())
            .field("primitives", &self.primitives.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::{AttribClass, AttribDefault, TypeQualifier};
    use approx::assert_relative_eq;

    #[test]
    fn empty_geometry() {
        let geo = Geometry::new();
        assert_eq!(geo.num_points(), 0);
        assert_eq!(geo.num_vertices(), 0);
        assert_eq!(geo.num_prims(), 0);
    }

    #[test]
    fn add_points() {
        let mut geo = Geometry::new();
        let h0 = geo.add_point(Vec3::new(1.0, 2.0, 3.0));
        let h1 = geo.add_point(Vec3::new(4.0, 5.0, 6.0));

        assert_eq!(geo.num_points(), 2);
        let p0 = geo.point_pos(h0);
        let p1 = geo.point_pos(h1);
        assert_relative_eq!(p0.x, 1.0);
        assert_relative_eq!(p1.z, 6.0);
    }

    #[test]
    fn add_triangle() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));

        let ph = geo.add_face(&[p0, p1, p2]);

        assert_eq!(geo.num_prims(), 1);
        assert_eq!(geo.num_vertices(), 3);

        let pts = geo.prim_points(ph);
        assert_eq!(pts, vec![p0, p1, p2]);
    }

    #[test]
    fn shared_points() {
        // Two triangles sharing an edge: p0-p1-p2 and p1-p3-p2
        // 4 unique points, 2 prims, 6 vertices total
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(1.0, 1.0, 0.0));
        let p3 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));

        geo.add_face(&[p0, p1, p2]);
        geo.add_face(&[p1, p3, p2]);

        assert_eq!(geo.num_points(), 4);
        assert_eq!(geo.num_prims(), 2);
        assert_eq!(geo.num_vertices(), 6);
    }

    #[test]
    fn bounding_box() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::new(-1.0, -2.0, -3.0));
        geo.add_point(Vec3::new(1.0, 2.0, 3.0));
        geo.add_point(Vec3::new(0.0, 0.0, 0.0));

        let bb = geo.bounding_box();
        assert!(bb.is_valid());
        assert_relative_eq!(bb.min.x, -1.0);
        assert_relative_eq!(bb.min.y, -2.0);
        assert_relative_eq!(bb.min.z, -3.0);
        assert_relative_eq!(bb.max.x, 1.0);
        assert_relative_eq!(bb.max.y, 2.0);
        assert_relative_eq!(bb.max.z, 3.0);
    }

    #[test]
    fn attributes_on_geometry() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::ZERO);
        let p1 = geo.add_point(Vec3::ONE);

        geo.add_attrib(
            AttribClass::Point,
            "Cd",
            AttribDefault::Vector3([1.0, 1.0, 1.0]),
            TypeQualifier::Color,
        )
        .unwrap();

        let handle: AttribHandle<[f32; 3]> =
            geo.find_attrib(AttribClass::Point, "Cd").unwrap();

        // Default values
        assert_eq!(geo.get_attrib(&handle, p0.index()).unwrap(), [1.0, 1.0, 1.0]);

        // Set a custom value
        geo.set_attrib(&handle, p1.index(), [0.5, 0.0, 0.0]).unwrap();
        assert_eq!(
            geo.get_attrib(&handle, p1.index()).unwrap(),
            [0.5, 0.0, 0.0]
        );
    }

    #[test]
    fn attributes_auto_resize_on_add_point() {
        let mut geo = Geometry::new();

        // Add attribute before any points
        geo.add_attrib(
            AttribClass::Point,
            "pscale",
            AttribDefault::Float(2.0),
            TypeQualifier::None,
        )
        .unwrap();

        // Add 3 points — storage should auto-resize
        geo.add_point(Vec3::ZERO);
        geo.add_point(Vec3::X);
        geo.add_point(Vec3::Y);

        let handle: AttribHandle<f32> = geo.find_attrib(AttribClass::Point, "pscale").unwrap();
        // All three elements should have the default value
        for i in 0..3 {
            assert_eq!(geo.get_attrib(&handle, i).unwrap(), 2.0, "index {i}");
        }
    }

    #[test]
    fn groups_on_geometry() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::ZERO);
        let p1 = geo.add_point(Vec3::X);
        let p2 = geo.add_point(Vec3::Y);

        geo.create_point_group("selection");
        geo.groups_mut()
            .point_group_mut("selection")
            .unwrap()
            .add(p0.index());
        geo.groups_mut()
            .point_group_mut("selection")
            .unwrap()
            .add(p2.index());

        assert!(geo.groups().point_group("selection").unwrap().contains(p0.index()));
        assert!(!geo.groups().point_group("selection").unwrap().contains(p1.index()));
        assert!(geo.groups().point_group("selection").unwrap().contains(p2.index()));
        assert_eq!(
            geo.groups().point_group("selection").unwrap().count(),
            2
        );
    }

    #[test]
    fn polyline() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(2.0, 0.0, 0.0));

        let ph = geo.add_polyline(&[p0, p1, p2]);

        let prim = geo.prim(ph);
        match prim {
            Primitive::Polygon(poly) => {
                assert_eq!(poly.poly_type, PolyType::Open);
                assert_eq!(poly.vertices.len(), 3);
            }
        }
    }
}
