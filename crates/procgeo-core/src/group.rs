use std::collections::{HashMap, HashSet};

use bitvec::prelude::*;

use crate::handle::PrimHandle;

// ---------------------------------------------------------------------------
// ElementGroup — bitset-backed membership set
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct ElementGroup {
    bits: BitVec,
}

impl ElementGroup {
    pub fn new(size: usize) -> Self {
        ElementGroup {
            bits: bitvec![0; size],
        }
    }

    pub fn contains(&self, index: usize) -> bool {
        self.bits.get(index).map(|b| *b).unwrap_or(false)
    }

    pub fn set(&mut self, index: usize, value: bool) {
        if index < self.bits.len() {
            self.bits.set(index, value);
        }
    }

    pub fn add(&mut self, index: usize) {
        self.set(index, true);
    }

    pub fn remove(&mut self, index: usize) {
        self.set(index, false);
    }

    pub fn count(&self) -> usize {
        self.bits.count_ones()
    }

    pub fn size(&self) -> usize {
        self.bits.len()
    }

    pub fn resize(&mut self, new_size: usize) {
        let old_size = self.bits.len();
        if new_size > old_size {
            self.bits.resize(new_size, false);
        } else {
            self.bits.truncate(new_size);
        }
    }

    /// Iterate over indices of set bits.
    pub fn iter_set(&self) -> impl Iterator<Item = usize> + '_ {
        self.bits
            .iter()
            .enumerate()
            .filter_map(|(i, b)| if *b { Some(i) } else { None })
    }

    /// Union: self |= other. Other is extended with false if shorter.
    pub fn union(&mut self, other: &ElementGroup) {
        let len = self.bits.len().max(other.bits.len());
        self.bits.resize(len, false);
        for i in 0..other.bits.len() {
            if other.bits[i] {
                self.bits.set(i, true);
            }
        }
    }

    /// Intersection: keep only bits set in both.
    pub fn intersect(&mut self, other: &ElementGroup) {
        for i in 0..self.bits.len() {
            let other_val = other.bits.get(i).map(|b| *b).unwrap_or(false);
            if !other_val {
                self.bits.set(i, false);
            }
        }
    }

    /// Subtraction: self &= !other.
    pub fn subtract(&mut self, other: &ElementGroup) {
        for i in 0..self.bits.len().min(other.bits.len()) {
            if other.bits[i] {
                self.bits.set(i, false);
            }
        }
    }

    /// Complement: flip all bits.
    pub fn complement(&mut self) {
        for i in 0..self.bits.len() {
            let v = self.bits[i];
            self.bits.set(i, !v);
        }
    }

    pub fn clear(&mut self) {
        self.bits.fill(false);
    }
}

// ---------------------------------------------------------------------------
// EdgeGroup — set of (prim, edge_index) pairs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct EdgeGroup {
    edges: HashSet<(PrimHandle, u8)>,
}

impl EdgeGroup {
    pub fn new() -> Self {
        EdgeGroup {
            edges: HashSet::new(),
        }
    }

    pub fn add(&mut self, prim: PrimHandle, edge_idx: u8) {
        self.edges.insert((prim, edge_idx));
    }

    pub fn remove(&mut self, prim: PrimHandle, edge_idx: u8) {
        self.edges.remove(&(prim, edge_idx));
    }

    pub fn contains(&self, prim: PrimHandle, edge_idx: u8) -> bool {
        self.edges.contains(&(prim, edge_idx))
    }

    pub fn count(&self) -> usize {
        self.edges.len()
    }

    pub fn clear(&mut self) {
        self.edges.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &(PrimHandle, u8)> {
        self.edges.iter()
    }
}

// ---------------------------------------------------------------------------
// GroupMap — manages all named groups
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct GroupMap {
    point_groups: HashMap<String, ElementGroup>,
    vertex_groups: HashMap<String, ElementGroup>,
    prim_groups: HashMap<String, ElementGroup>,
    edge_groups: HashMap<String, EdgeGroup>,
}

impl GroupMap {
    pub fn new() -> Self {
        GroupMap::default()
    }

    // --- Point groups -------------------------------------------------------

    pub fn create_point_group(&mut self, name: impl Into<String>, size: usize) {
        self.point_groups
            .entry(name.into())
            .or_insert_with(|| ElementGroup::new(size));
    }

    pub fn point_group(&self, name: &str) -> Option<&ElementGroup> {
        self.point_groups.get(name)
    }

    pub fn point_group_mut(&mut self, name: &str) -> Option<&mut ElementGroup> {
        self.point_groups.get_mut(name)
    }

    pub fn delete_point_group(&mut self, name: &str) -> bool {
        self.point_groups.remove(name).is_some()
    }

    pub fn point_group_names(&self) -> Vec<&str> {
        self.point_groups.keys().map(|s| s.as_str()).collect()
    }

    pub fn resize_point_groups(&mut self, new_size: usize) {
        for g in self.point_groups.values_mut() {
            g.resize(new_size);
        }
    }

    // --- Vertex groups ------------------------------------------------------

    pub fn create_vertex_group(&mut self, name: impl Into<String>, size: usize) {
        self.vertex_groups
            .entry(name.into())
            .or_insert_with(|| ElementGroup::new(size));
    }

    pub fn vertex_group(&self, name: &str) -> Option<&ElementGroup> {
        self.vertex_groups.get(name)
    }

    pub fn vertex_group_mut(&mut self, name: &str) -> Option<&mut ElementGroup> {
        self.vertex_groups.get_mut(name)
    }

    pub fn delete_vertex_group(&mut self, name: &str) -> bool {
        self.vertex_groups.remove(name).is_some()
    }

    pub fn vertex_group_names(&self) -> Vec<&str> {
        self.vertex_groups.keys().map(|s| s.as_str()).collect()
    }

    pub fn resize_vertex_groups(&mut self, new_size: usize) {
        for g in self.vertex_groups.values_mut() {
            g.resize(new_size);
        }
    }

    // --- Primitive groups ---------------------------------------------------

    pub fn create_prim_group(&mut self, name: impl Into<String>, size: usize) {
        self.prim_groups
            .entry(name.into())
            .or_insert_with(|| ElementGroup::new(size));
    }

    pub fn prim_group(&self, name: &str) -> Option<&ElementGroup> {
        self.prim_groups.get(name)
    }

    pub fn prim_group_mut(&mut self, name: &str) -> Option<&mut ElementGroup> {
        self.prim_groups.get_mut(name)
    }

    pub fn delete_prim_group(&mut self, name: &str) -> bool {
        self.prim_groups.remove(name).is_some()
    }

    pub fn prim_group_names(&self) -> Vec<&str> {
        self.prim_groups.keys().map(|s| s.as_str()).collect()
    }

    pub fn resize_prim_groups(&mut self, new_size: usize) {
        for g in self.prim_groups.values_mut() {
            g.resize(new_size);
        }
    }

    // --- Edge groups --------------------------------------------------------

    pub fn create_edge_group(&mut self, name: impl Into<String>) {
        self.edge_groups
            .entry(name.into())
            .or_default();
    }

    pub fn edge_group(&self, name: &str) -> Option<&EdgeGroup> {
        self.edge_groups.get(name)
    }

    pub fn edge_group_mut(&mut self, name: &str) -> Option<&mut EdgeGroup> {
        self.edge_groups.get_mut(name)
    }

    pub fn delete_edge_group(&mut self, name: &str) -> bool {
        self.edge_groups.remove(name).is_some()
    }

    pub fn edge_group_names(&self) -> Vec<&str> {
        self.edge_groups.keys().map(|s| s.as_str()).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_group_basic() {
        let mut g = ElementGroup::new(10);
        g.add(2);
        g.add(5);
        g.add(7);

        assert!(g.contains(2));
        assert!(g.contains(5));
        assert!(!g.contains(0));
        assert_eq!(g.count(), 3);

        g.remove(5);
        assert!(!g.contains(5));
        assert_eq!(g.count(), 2);
    }

    #[test]
    fn group_boolean_ops() {
        // a = {0,1,2,3,4,5}
        let mut a = ElementGroup::new(10);
        for i in 0..6 {
            a.add(i);
        }

        // b = {2,3,8,9}
        let mut b = ElementGroup::new(10);
        b.add(2);
        b.add(3);
        b.add(8);
        b.add(9);

        // union
        let mut u = a.clone();
        u.union(&b);
        assert_eq!(u.count(), 8); // 0..5 + 8,9

        // intersect
        let mut i = a.clone();
        i.intersect(&b);
        assert_eq!(i.count(), 2); // 2,3

        // subtract
        let mut s = a.clone();
        s.subtract(&b);
        assert_eq!(s.count(), 4); // 0,1,4,5
    }

    #[test]
    fn group_complement() {
        let mut g = ElementGroup::new(8);
        g.add(0);
        g.add(1);
        g.add(2);
        assert_eq!(g.count(), 3);

        g.complement();
        assert_eq!(g.count(), 5); // 3,4,5,6,7
        assert!(!g.contains(0));
        assert!(g.contains(3));
        assert!(g.contains(7));
    }

    #[test]
    fn group_iter_set() {
        let mut g = ElementGroup::new(12);
        g.add(2);
        g.add(5);
        g.add(8);

        let indices: Vec<usize> = g.iter_set().collect();
        assert_eq!(indices, vec![2, 5, 8]);
    }

    #[test]
    fn edge_group_basic() {
        let mut eg = EdgeGroup::new();
        let p0 = PrimHandle::from_index(0);
        let p1 = PrimHandle::from_index(1);

        eg.add(p0, 0);
        eg.add(p0, 1);
        eg.add(p1, 2);

        assert!(eg.contains(p0, 0));
        assert!(eg.contains(p0, 1));
        assert!(eg.contains(p1, 2));
        assert!(!eg.contains(p1, 0));
        assert_eq!(eg.count(), 3);

        eg.remove(p0, 1);
        assert!(!eg.contains(p0, 1));
        assert_eq!(eg.count(), 2);
    }

    #[test]
    fn group_map() {
        let mut gm = GroupMap::new();
        gm.create_point_group("mypts", 10);
        gm.create_prim_group("myprims", 5);

        {
            let pg = gm.point_group_mut("mypts").unwrap();
            pg.add(3);
            pg.add(7);
        }
        {
            let prg = gm.prim_group_mut("myprims").unwrap();
            prg.add(1);
        }

        assert!(gm.point_group("mypts").unwrap().contains(3));
        assert!(gm.point_group("mypts").unwrap().contains(7));
        assert_eq!(gm.point_group("mypts").unwrap().count(), 2);

        assert!(gm.prim_group("myprims").unwrap().contains(1));
        assert_eq!(gm.prim_group("myprims").unwrap().count(), 1);

        assert!(gm.point_group("nonexistent").is_none());
    }

    #[test]
    fn group_resize() {
        let mut g = ElementGroup::new(4);
        g.add(0);
        g.add(3);
        assert_eq!(g.count(), 2);

        // Expand to 8
        g.resize(8);
        assert_eq!(g.size(), 8);
        assert!(g.contains(0));
        assert!(g.contains(3));
        assert!(!g.contains(4));
        assert_eq!(g.count(), 2);
    }
}
