use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PolyType, PrimHandle, Primitive};

use crate::{Sop, SopError};

/// Parameters for the Softbody SOP / [`SoftbodySolver`].
///
/// The solver is an XPBD (Extended Position Based Dynamics) cloth/softbody
/// integrator. It works on *any* geometry: distance constraints are derived
/// from polygon edges (structural) and polygon diagonals (bend/shear), so the
/// same solver drives cloth grids, closed meshes, polylines, and even raw
/// point clouds (which simply fall under gravity).
///
/// Defaults are tuned to be a stable, gently stiff cloth at 24 fps and mirror
/// the kind of values you would reach for in Houdini's Vellum.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoftbodyParams {
    /// Frame to simulate up to. Frame `0` is the rest state (pass-through).
    /// The [`SoftbodySop`] re-simulates from rest to this frame on every call.
    pub frame: u32,
    /// Frames per second. Controls the per-frame timestep `1.0 / fps`.
    pub fps: f32,
    /// Number of integration substeps per frame. More substeps = more stable
    /// and stiffer constraints for the same iteration count.
    pub substeps: u32,
    /// Constraint solver (Gauss–Seidel) iterations per substep.
    pub iterations: u32,
    /// Constant gravitational acceleration (world units / s²).
    pub gravity: Vec3,
    /// Structural (edge) stiffness in `[0, 1]`. `1.0` is rigid, `0.0` is fully
    /// compliant.
    pub stiffness: f32,
    /// Bend / shear stiffness in `[0, 1]` applied across polygon diagonals.
    /// `0.0` disables bend constraints entirely.
    pub bend_stiffness: f32,
    /// Per-substep velocity damping in `[0, 1]`. `0.0` = no damping.
    pub damping: f32,
    /// Mass of every (unpinned) point. Larger mass resists forces more.
    pub mass: f32,
    /// Optional point group whose points are pinned (infinite mass / fixed).
    pub pin_group: Option<String>,
    /// Enable collision against an infinite horizontal ground plane.
    pub ground_collision: bool,
    /// Height (world Y) of the ground plane.
    pub ground_height: f32,
    /// Tangential friction `[0, 1]` applied on ground contact.
    pub ground_friction: f32,
    /// Constant wind force (world units / s²) added to every unpinned point.
    pub wind: Vec3,
}

impl Default for SoftbodyParams {
    fn default() -> Self {
        SoftbodyParams {
            frame: 0,
            fps: 24.0,
            substeps: 5,
            iterations: 8,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            stiffness: 0.9,
            bend_stiffness: 0.2,
            damping: 0.02,
            mass: 1.0,
            pin_group: None,
            ground_collision: true,
            ground_height: 0.0,
            ground_friction: 0.3,
            wind: Vec3::ZERO,
        }
    }
}

/// Map an artist-friendly stiffness in `[0, 1]` to an XPBD compliance value
/// (inverse stiffness, units of m/N). `1.0` → `0.0` (perfectly rigid).
#[inline]
fn compliance_from_stiffness(stiffness: f32) -> f32 {
    let s = stiffness.clamp(0.0, 1.0);
    // Quadratic falloff keeps the high end (stiff cloth) feeling responsive
    // while still allowing very soft behaviour near zero.
    (1.0 - s) * (1.0 - s) * 1.0e-2
}

#[derive(Clone, Copy)]
struct DistanceConstraint {
    a: usize,
    b: usize,
    rest: f32,
    compliance: f32,
}

/// A stateful XPBD softbody/cloth solver.
///
/// Build it once from a rest geometry, then [`step`](Self::step) it frame by
/// frame. The topology never changes, so realtime viewers can cache only the
/// per-frame point positions for scrubbing.
#[derive(Clone)]
pub struct SoftbodySolver {
    /// Rest geometry, used as the template for [`geometry`](Self::geometry).
    template: Geometry,
    /// Rest positions (frame 0), kept for [`reset`](Self::reset).
    rest: Vec<Vec3>,
    pos: Vec<Vec3>,
    prev: Vec<Vec3>,
    vel: Vec<Vec3>,
    /// Inverse mass per point. `0.0` marks a pinned/fixed point.
    inv_mass: Vec<f32>,
    constraints: Vec<DistanceConstraint>,
    /// Per-constraint Lagrange multiplier accumulator (reset each substep).
    lambda: Vec<f32>,
    params: SoftbodyParams,
    frame: u32,
}

impl SoftbodySolver {
    /// Build a solver from a rest geometry and parameters.
    pub fn new(geo: &Geometry, params: &SoftbodyParams) -> Self {
        let n = geo.num_points();
        let rest: Vec<Vec3> = geo.points().collect();
        let pos = rest.clone();
        let prev = rest.clone();
        let vel = vec![Vec3::ZERO; n];

        let base_inv_mass = if params.mass > 0.0 {
            1.0 / params.mass
        } else {
            0.0
        };
        let mut inv_mass = vec![base_inv_mass; n];

        // Pin points belonging to the pin group (infinite mass).
        if let Some(name) = params.pin_group.as_ref() {
            if !name.is_empty() {
                if let Some(group) = geo.groups().point_group(name) {
                    for (i, m) in inv_mass.iter_mut().enumerate() {
                        if group.contains(i) {
                            *m = 0.0;
                        }
                    }
                }
            }
        }

        let constraints = Self::build_constraints(geo, &rest, params);
        let lambda = vec![0.0; constraints.len()];

        SoftbodySolver {
            template: geo.clone(),
            rest,
            pos,
            prev,
            vel,
            inv_mass,
            constraints,
            lambda,
            params: params.clone(),
            frame: 0,
        }
    }

    /// Derive distance constraints from polygon edges (structural) and polygon
    /// diagonals (bend / shear). De-duplicated, with structural compliance
    /// winning over bend compliance on shared keys.
    fn build_constraints(
        geo: &Geometry,
        rest: &[Vec3],
        params: &SoftbodyParams,
    ) -> Vec<DistanceConstraint> {
        let structural = compliance_from_stiffness(params.stiffness);
        let bend = compliance_from_stiffness(params.bend_stiffness);

        // edge key (min, max) -> compliance
        let mut edges: HashMap<(usize, usize), f32> = HashMap::new();
        let key = |a: usize, b: usize| if a < b { (a, b) } else { (b, a) };

        for pi in 0..geo.num_prims() {
            let ph = PrimHandle::from_index(pi);
            let pts = geo.prim_points(ph);
            let cnt = pts.len();
            if cnt < 2 {
                continue;
            }
            let closed = matches!(geo.prim(ph), Primitive::Polygon(p) if p.poly_type == PolyType::Closed);
            let edge_count = if closed { cnt } else { cnt - 1 };

            // Structural edges.
            for i in 0..edge_count {
                let a = pts[i].index();
                let b = pts[(i + 1) % cnt].index();
                if a != b {
                    edges.entry(key(a, b)).or_insert(structural);
                }
            }

            // Bend / shear diagonals (only meaningful for faces with >= 4 sides).
            if params.bend_stiffness > 0.0 && cnt >= 4 {
                let diag_count = if closed { cnt } else { cnt - 2 };
                for i in 0..diag_count {
                    let a = pts[i].index();
                    let b = pts[(i + 2) % cnt].index();
                    if a != b {
                        edges.entry(key(a, b)).or_insert(bend);
                    }
                }
            }
        }

        edges
            .into_iter()
            .map(|((a, b), compliance)| DistanceConstraint {
                a,
                b,
                rest: (rest[a] - rest[b]).length(),
                compliance,
            })
            .collect()
    }

    /// Number of simulated points.
    pub fn num_points(&self) -> usize {
        self.pos.len()
    }

    /// Number of internal distance constraints.
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// The current simulated frame (`0` = rest).
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Current point positions.
    pub fn positions(&self) -> &[Vec3] {
        &self.pos
    }

    /// Reset the solver back to the rest state (frame 0).
    pub fn reset(&mut self) {
        self.pos.copy_from_slice(&self.rest);
        self.prev.copy_from_slice(&self.rest);
        for v in &mut self.vel {
            *v = Vec3::ZERO;
        }
        self.frame = 0;
    }

    /// Advance the simulation by a single frame.
    pub fn step(&mut self) {
        let dt = if self.params.fps > 0.0 {
            1.0 / self.params.fps
        } else {
            1.0 / 24.0
        };
        let substeps = self.params.substeps.max(1);
        let sub_dt = dt / substeps as f32;
        for _ in 0..substeps {
            self.substep(sub_dt);
        }
        self.frame += 1;
    }

    /// Run the simulation from the current state up to `frame`. If `frame` is at
    /// or before the current frame, the solver is reset first.
    pub fn solve_to(&mut self, frame: u32) {
        if frame <= self.frame {
            self.reset();
        }
        while self.frame < frame {
            self.step();
        }
    }

    fn substep(&mut self, dt: f32) {
        let accel = self.params.gravity + self.params.wind;
        let damping = 1.0 - self.params.damping.clamp(0.0, 1.0);

        // Integrate (predict positions).
        for i in 0..self.pos.len() {
            if self.inv_mass[i] == 0.0 {
                continue;
            }
            self.vel[i] += accel * dt;
            self.vel[i] *= damping;
            self.prev[i] = self.pos[i];
            self.pos[i] += self.vel[i] * dt;
        }

        // Solve XPBD distance constraints.
        let alpha_scale = 1.0 / (dt * dt);
        for l in &mut self.lambda {
            *l = 0.0;
        }
        let iterations = self.params.iterations.max(1);
        for _ in 0..iterations {
            for (ci, c) in self.constraints.iter().enumerate() {
                let wa = self.inv_mass[c.a];
                let wb = self.inv_mass[c.b];
                let w = wa + wb;
                if w == 0.0 {
                    continue;
                }
                let delta = self.pos[c.a] - self.pos[c.b];
                let len = delta.length();
                if len < 1.0e-9 {
                    continue;
                }
                let dir = delta / len;
                let alpha = c.compliance * alpha_scale;
                let constraint = len - c.rest;
                let d_lambda = (-constraint - alpha * self.lambda[ci]) / (w + alpha);
                self.lambda[ci] += d_lambda;
                let correction = dir * d_lambda;
                if wa > 0.0 {
                    self.pos[c.a] += correction * wa;
                }
                if wb > 0.0 {
                    self.pos[c.b] -= correction * wb;
                }
            }

            if self.params.ground_collision {
                self.resolve_ground();
            }
        }

        // Update velocities from the position change.
        for i in 0..self.pos.len() {
            if self.inv_mass[i] == 0.0 {
                self.vel[i] = Vec3::ZERO;
                continue;
            }
            self.vel[i] = (self.pos[i] - self.prev[i]) / dt;
        }
    }

    fn resolve_ground(&mut self) {
        let ground = self.params.ground_height;
        let friction = self.params.ground_friction.clamp(0.0, 1.0);
        for i in 0..self.pos.len() {
            if self.inv_mass[i] == 0.0 {
                continue;
            }
            if self.pos[i].y < ground {
                self.pos[i].y = ground;
                // Tangential friction: damp horizontal motion relative to prev.
                if friction > 0.0 {
                    let dx = self.pos[i].x - self.prev[i].x;
                    let dz = self.pos[i].z - self.prev[i].z;
                    self.pos[i].x -= dx * friction;
                    self.pos[i].z -= dz * friction;
                }
            }
        }
    }

    /// Return a geometry snapshot at the current frame: the original topology
    /// and attributes with point positions replaced by the simulated state.
    pub fn geometry(&self) -> Geometry {
        let mut geo = self.template.clone();
        for (i, p) in self.pos.iter().enumerate() {
            geo.set_point_pos(PointHandle::from_index(i), *p);
        }
        geo
    }
}

/// Softbody SOP — a stateless wrapper around [`SoftbodySolver`].
///
/// Simulates the input geometry from rest up to `params.frame` and returns the
/// deformed geometry at that frame. For interactive playback, drive a
/// [`SoftbodySolver`] directly and cache its per-frame positions instead of
/// re-running this SOP every frame.
pub struct SoftbodySop;

impl Sop for SoftbodySop {
    type Params = SoftbodyParams;

    fn name(&self) -> &'static str {
        "softbody"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        // Frame 0 is the rest state — nothing to simulate.
        if params.frame == 0 || geo.num_points() == 0 {
            return Ok(geo.clone());
        }

        let mut solver = SoftbodySolver::new(geo, params);
        solver.solve_to(params.frame);
        Ok(solver.geometry())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::grid::{GridParams, GridSop};
    use crate::{GeometryExt, generate};

    fn make_grid() -> Geometry {
        generate(
            &GridSop,
            &GridParams {
                rows: 10,
                cols: 10,
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn softbody_frame_zero_is_passthrough() {
        let geo = make_grid();
        let before: Vec<Vec3> = geo.points().collect();
        let result = geo
            .apply(&SoftbodySop, &SoftbodyParams::default())
            .unwrap();
        let after: Vec<Vec3> = result.points().collect();
        assert_eq!(before, after);
    }

    #[test]
    fn softbody_preserves_topology() {
        let geo = make_grid();
        let (np, nv, npr) = (geo.num_points(), geo.num_vertices(), geo.num_prims());
        let params = SoftbodyParams {
            frame: 5,
            ..Default::default()
        };
        let result = geo.apply(&SoftbodySop, &params).unwrap();
        assert_eq!(result.num_points(), np);
        assert_eq!(result.num_vertices(), nv);
        assert_eq!(result.num_prims(), npr);
    }

    #[test]
    fn softbody_falls_under_gravity() {
        // A grid on the XZ plane (y=0) with no pins and the ground far below
        // should drop in average height after simulating a few frames.
        let geo = make_grid();
        let params = SoftbodyParams {
            frame: 10,
            ground_collision: false,
            ..Default::default()
        };
        let avg_y_before =
            geo.points().map(|p| p.y).sum::<f32>() / geo.num_points() as f32;
        let result = geo.apply(&SoftbodySop, &params).unwrap();
        let avg_y_after =
            result.points().map(|p| p.y).sum::<f32>() / result.num_points() as f32;
        assert!(
            avg_y_after < avg_y_before - 0.1,
            "geometry should fall: before={avg_y_before}, after={avg_y_after}"
        );
    }

    #[test]
    fn softbody_pin_group_holds() {
        // Pin every point: the cloth must not move at all.
        let mut geo = make_grid();
        let n = geo.num_points();
        geo.create_point_group("pinned");
        {
            let group = geo.groups_mut().point_group_mut("pinned").unwrap();
            for i in 0..n {
                group.add(i);
            }
        }
        let params = SoftbodyParams {
            frame: 20,
            pin_group: Some("pinned".to_string()),
            ..Default::default()
        };
        let before: Vec<Vec3> = geo.points().collect();
        let result = geo.apply(&SoftbodySop, &params).unwrap();
        let after: Vec<Vec3> = result.points().collect();
        for (b, a) in before.iter().zip(after.iter()) {
            assert!((*b - *a).length() < 1e-5, "pinned points must not move");
        }
    }

    #[test]
    fn softbody_ground_collision_clamps() {
        // With the ground at y=0 and gravity pulling down, no point should end
        // up meaningfully below the ground plane.
        let geo = make_grid();
        let params = SoftbodyParams {
            frame: 30,
            ground_collision: true,
            ground_height: 0.0,
            ..Default::default()
        };
        let result = geo.apply(&SoftbodySop, &params).unwrap();
        for p in result.points() {
            assert!(p.y > -0.05, "point fell through ground: y={}", p.y);
        }
    }

    #[test]
    fn softbody_empty_geometry() {
        let geo = Geometry::new();
        let params = SoftbodyParams {
            frame: 5,
            ..Default::default()
        };
        let result = geo.apply(&SoftbodySop, &params).unwrap();
        assert_eq!(result.num_points(), 0);
    }

    #[test]
    fn solver_step_matches_solve_to() {
        let geo = make_grid();
        let params = SoftbodyParams {
            frame: 7,
            ..Default::default()
        };

        let mut a = SoftbodySolver::new(&geo, &params);
        for _ in 0..7 {
            a.step();
        }

        let mut b = SoftbodySolver::new(&geo, &params);
        b.solve_to(7);

        assert_eq!(a.frame(), b.frame());
        for (pa, pb) in a.positions().iter().zip(b.positions().iter()) {
            assert!((*pa - *pb).length() < 1e-6);
        }
    }

    #[test]
    fn solver_reset_restores_rest() {
        let geo = make_grid();
        let rest: Vec<Vec3> = geo.points().collect();
        let mut solver = SoftbodySolver::new(&geo, &SoftbodyParams::default());
        for _ in 0..10 {
            solver.step();
        }
        solver.reset();
        assert_eq!(solver.frame(), 0);
        for (r, p) in rest.iter().zip(solver.positions().iter()) {
            assert!((*r - *p).length() < 1e-6);
        }
    }

    #[test]
    fn solver_builds_constraints_from_edges() {
        let geo = make_grid();
        let solver = SoftbodySolver::new(&geo, &SoftbodyParams::default());
        assert!(
            solver.num_constraints() > 0,
            "grid should yield distance constraints"
        );
    }
}
