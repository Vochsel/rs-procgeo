use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{
    AttribClass, AttribDefault, AttribHandle, AttribType, CoreError, Geometry, PrimHandle,
    Primitive, TypeQualifier, VertexHandle,
};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalGroupType {
    #[default]
    GuessFromGroup,
    Points,
    Vertices,
    Primitives,
    Edges,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalTarget {
    #[default]
    Points,
    Vertices,
    Primitives,
    Detail,
}

impl NormalTarget {
    fn attrib_class(self) -> AttribClass {
        match self {
            NormalTarget::Points => AttribClass::Point,
            NormalTarget::Vertices => AttribClass::Vertex,
            NormalTarget::Primitives => AttribClass::Primitive,
            NormalTarget::Detail => AttribClass::Detail,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalWeightingMethod {
    #[default]
    ByVertexAngle,
    EachVertexEqually,
    ByFaceArea,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalParams {
    pub group: String,
    pub group_type: NormalGroupType,
    pub override_normal: String,
    pub compute_normals: bool,
    pub add_normals_to: NormalTarget,
    pub cusp_angle: f32,
    pub weighting_method: NormalWeightingMethod,
    pub keep_original_zero: bool,
    pub make_unit_length: bool,
    pub reverse_normals: bool,
}

impl Default for NormalParams {
    fn default() -> Self {
        Self {
            group: String::new(),
            group_type: NormalGroupType::GuessFromGroup,
            override_normal: "N".to_string(),
            compute_normals: true,
            add_normals_to: NormalTarget::Points,
            cusp_angle: 60.0,
            weighting_method: NormalWeightingMethod::ByVertexAngle,
            keep_original_zero: false,
            make_unit_length: false,
            reverse_normals: false,
        }
    }
}

pub struct NormalSop;

#[derive(Clone, Copy, Debug)]
struct VertexNormalSample {
    vertex: VertexHandle,
    face_normal: Vec3,
    area: f32,
    corner_angle: f32,
}

#[derive(Clone, Debug)]
struct NormalContext {
    prim_normals: Vec<Vec3>,
    prim_areas: Vec<f32>,
    point_samples: Vec<Vec<VertexNormalSample>>,
}

#[derive(Clone, Debug)]
struct SelectionMasks {
    points: Vec<bool>,
    vertices: Vec<bool>,
    prims: Vec<bool>,
}

impl SelectionMasks {
    fn all(geo: &Geometry) -> Self {
        Self {
            points: vec![true; geo.num_points()],
            vertices: vec![true; geo.num_vertices()],
            prims: vec![true; geo.num_prims()],
        }
    }

    fn for_target(&self, target: NormalTarget, geo: &Geometry) -> Vec<bool> {
        match target {
            NormalTarget::Points => self.points.clone(),
            NormalTarget::Vertices => self.vertices.clone(),
            NormalTarget::Primitives => self.prims.clone(),
            NormalTarget::Detail => {
                let _ = geo;
                vec![true]
            }
        }
    }
}

fn newell_sum(positions: &[Vec3]) -> Vec3 {
    let n = positions.len();
    let mut nx = 0.0_f32;
    let mut ny = 0.0_f32;
    let mut nz = 0.0_f32;
    for i in 0..n {
        let cur = positions[i];
        let next = positions[(i + 1) % n];
        nx += (cur.y - next.y) * (cur.z + next.z);
        ny += (cur.z - next.z) * (cur.x + next.x);
        nz += (cur.x - next.x) * (cur.y + next.y);
    }
    Vec3::new(nx, ny, nz)
}

fn corner_angle(prev: Vec3, current: Vec3, next: Vec3) -> f32 {
    let a = (prev - current).normalize_or_zero();
    let b = (next - current).normalize_or_zero();
    if a.length_squared() < 1e-10 || b.length_squared() < 1e-10 {
        return 0.0;
    }
    a.dot(b).clamp(-1.0, 1.0).acos()
}

fn sample_weight(sample: &VertexNormalSample, method: NormalWeightingMethod) -> f32 {
    match method {
        NormalWeightingMethod::ByVertexAngle => sample.corner_angle,
        NormalWeightingMethod::EachVertexEqually => 1.0,
        NormalWeightingMethod::ByFaceArea => sample.area,
    }
}

fn normal_attr_exists(geo: &Geometry, class: AttribClass, name: &str) -> Result<bool, SopError> {
    match geo.attrib_type(class, name) {
        None => Ok(false),
        Some(AttribType::Vector3) => Ok(true),
        Some(other) => Err(SopError::Core(CoreError::AttributeTypeMismatch(format!(
            "attribute {name} on {class:?} must be Vector3, got {other:?}"
        )))),
    }
}

fn ensure_normal_attrib(
    geo: &mut Geometry,
    class: AttribClass,
    name: &str,
) -> Result<AttribHandle<[f32; 3]>, SopError> {
    if !normal_attr_exists(geo, class, name)? {
        geo.add_attrib(
            class,
            name,
            AttribDefault::Vector3([0.0, 0.0, 0.0]),
            TypeQualifier::Normal,
        )
        .map_err(SopError::Core)?;
    }
    geo.find_attrib::<[f32; 3]>(class, name)
        .map_err(SopError::Core)
}

fn build_normal_context(geo: &Geometry) -> NormalContext {
    let mut prim_normals = vec![Vec3::ZERO; geo.num_prims()];
    let mut prim_areas = vec![0.0_f32; geo.num_prims()];
    let mut point_samples = vec![Vec::new(); geo.num_points()];

    for prim_idx in 0..geo.num_prims() {
        let prim_handle = PrimHandle::from_index(prim_idx);
        let prim = geo.prim(prim_handle);
        let prim_vertices = geo.prim_vertices(prim_handle);
        let positions: Vec<Vec3> = prim_vertices
            .iter()
            .map(|&vh| geo.point_pos(geo.vertex_point(vh)))
            .collect();

        let (face_normal, area) = match prim {
            Primitive::Polygon(poly) if poly.poly_type == procgeo_core::PolyType::Closed => {
                let sum = if positions.len() >= 3 {
                    newell_sum(&positions)
                } else {
                    Vec3::ZERO
                };
                (sum.normalize_or_zero(), sum.length() * 0.5)
            }
            _ => (Vec3::ZERO, 0.0),
        };

        prim_normals[prim_idx] = face_normal;
        prim_areas[prim_idx] = area;

        let closed = matches!(
            prim,
            Primitive::Polygon(poly) if poly.poly_type == procgeo_core::PolyType::Closed
        );
        for (corner_idx, &vertex) in prim_vertices.iter().enumerate() {
            let point = geo.vertex_point(vertex);
            let angle = if closed && positions.len() >= 3 {
                let prev = positions[(corner_idx + positions.len() - 1) % positions.len()];
                let current = positions[corner_idx];
                let next = positions[(corner_idx + 1) % positions.len()];
                corner_angle(prev, current, next)
            } else {
                0.0
            };

            point_samples[point.index()].push(VertexNormalSample {
                vertex,
                face_normal,
                area,
                corner_angle: angle,
            });
        }
    }

    NormalContext {
        prim_normals,
        prim_areas,
        point_samples,
    }
}

fn compute_point_normals(ctx: &NormalContext, method: NormalWeightingMethod) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; ctx.point_samples.len()];

    for (point_idx, samples) in ctx.point_samples.iter().enumerate() {
        let mut sum = Vec3::ZERO;
        for sample in samples {
            let weight = sample_weight(sample, method);
            sum += sample.face_normal * weight;
        }
        normals[point_idx] = sum.normalize_or_zero();
    }

    normals
}

fn compute_vertex_normals(
    ctx: &NormalContext,
    method: NormalWeightingMethod,
    cusp_angle: f32,
) -> Vec<Vec3> {
    let cusp_cos = cusp_angle.clamp(0.0, 180.0).to_radians().cos();
    let num_vertices = ctx
        .point_samples
        .iter()
        .flat_map(|samples| samples.iter().map(|sample| sample.vertex.index()))
        .max()
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut normals = vec![Vec3::ZERO; num_vertices];

    for samples in &ctx.point_samples {
        for sample in samples {
            let mut sum = Vec3::ZERO;
            for other in samples {
                if sample.face_normal.length_squared() < 1e-10
                    || other.face_normal.length_squared() < 1e-10
                {
                    continue;
                }
                if sample.face_normal.dot(other.face_normal) >= cusp_cos - 1e-6 {
                    sum += other.face_normal * sample_weight(other, method);
                }
            }
            normals[sample.vertex.index()] = sum.normalize_or_zero();
        }
    }

    normals
}

fn compute_detail_normal(ctx: &NormalContext, prim_mask: &[bool]) -> Vec3 {
    let mut sum = Vec3::ZERO;
    for prim_idx in 0..ctx.prim_normals.len() {
        if prim_idx < prim_mask.len() && prim_mask[prim_idx] {
            sum += ctx.prim_normals[prim_idx] * ctx.prim_areas[prim_idx];
        }
    }
    sum.normalize_or_zero()
}

fn resolve_group_type(
    geo: &Geometry,
    group: &str,
    group_type: NormalGroupType,
) -> Result<NormalGroupType, SopError> {
    if group.is_empty() {
        return Ok(group_type);
    }

    let has_points = geo.groups().point_group(group).is_some();
    let has_vertices = geo.groups().vertex_group(group).is_some();
    let has_prims = geo.groups().prim_group(group).is_some();
    let has_edges = geo.groups().edge_group(group).is_some();

    match group_type {
        NormalGroupType::GuessFromGroup => {
            if !has_points && !has_vertices && !has_prims && !has_edges {
                return Err(SopError::Core(CoreError::GroupNotFound(group.to_string())));
            }

            let matches = has_points as u8 + has_vertices as u8 + has_prims as u8 + has_edges as u8;
            if matches > 1 && has_prims {
                Ok(NormalGroupType::Primitives)
            } else if has_points {
                Ok(NormalGroupType::Points)
            } else if has_vertices {
                Ok(NormalGroupType::Vertices)
            } else if has_prims {
                Ok(NormalGroupType::Primitives)
            } else {
                Ok(NormalGroupType::Edges)
            }
        }
        NormalGroupType::Points if has_points => Ok(NormalGroupType::Points),
        NormalGroupType::Vertices if has_vertices => Ok(NormalGroupType::Vertices),
        NormalGroupType::Primitives if has_prims => Ok(NormalGroupType::Primitives),
        NormalGroupType::Edges if has_edges => Ok(NormalGroupType::Edges),
        _ => Err(SopError::Core(CoreError::GroupNotFound(group.to_string()))),
    }
}

fn selection_from_group(
    geo: &Geometry,
    group: &str,
    group_type: NormalGroupType,
) -> Result<SelectionMasks, SopError> {
    let mut masks = SelectionMasks {
        points: vec![false; geo.num_points()],
        vertices: vec![false; geo.num_vertices()],
        prims: vec![false; geo.num_prims()],
    };

    match resolve_group_type(geo, group, group_type)? {
        NormalGroupType::Points => {
            let group = geo
                .groups()
                .point_group(group)
                .ok_or_else(|| SopError::Core(CoreError::GroupNotFound(group.to_string())))?;

            for point_idx in group.iter_set() {
                if point_idx < masks.points.len() {
                    masks.points[point_idx] = true;
                }
            }

            for vertex_idx in 0..geo.num_vertices() {
                let vertex = VertexHandle::from_index(vertex_idx);
                let point_idx = geo.vertex_point(vertex).index();
                if point_idx < masks.points.len() && masks.points[point_idx] {
                    masks.vertices[vertex_idx] = true;
                    masks.prims[geo.vertex_prim(vertex).index()] = true;
                }
            }
        }
        NormalGroupType::Vertices => {
            let group = geo
                .groups()
                .vertex_group(group)
                .ok_or_else(|| SopError::Core(CoreError::GroupNotFound(group.to_string())))?;

            for vertex_idx in group.iter_set() {
                if vertex_idx < masks.vertices.len() {
                    let vertex = VertexHandle::from_index(vertex_idx);
                    masks.vertices[vertex_idx] = true;
                    masks.points[geo.vertex_point(vertex).index()] = true;
                    masks.prims[geo.vertex_prim(vertex).index()] = true;
                }
            }
        }
        NormalGroupType::Primitives => {
            let group = geo
                .groups()
                .prim_group(group)
                .ok_or_else(|| SopError::Core(CoreError::GroupNotFound(group.to_string())))?;

            for prim_idx in group.iter_set() {
                if prim_idx < masks.prims.len() {
                    masks.prims[prim_idx] = true;
                    let prim = PrimHandle::from_index(prim_idx);
                    for &vertex in geo.prim_vertices(prim) {
                        masks.vertices[vertex.index()] = true;
                        masks.points[geo.vertex_point(vertex).index()] = true;
                    }
                }
            }
        }
        NormalGroupType::Edges => {
            let group = geo
                .groups()
                .edge_group(group)
                .ok_or_else(|| SopError::Core(CoreError::GroupNotFound(group.to_string())))?;

            for &(prim, edge_idx) in group.iter() {
                let vertices = geo.prim_vertices(prim);
                if vertices.is_empty() {
                    continue;
                }

                let edge_idx = edge_idx as usize;
                if edge_idx >= vertices.len() {
                    continue;
                }

                let next_idx = match geo.prim(prim) {
                    Primitive::Polygon(poly) if poly.poly_type == procgeo_core::PolyType::Open => {
                        if edge_idx + 1 >= vertices.len() {
                            continue;
                        }
                        edge_idx + 1
                    }
                    _ => (edge_idx + 1) % vertices.len(),
                };

                let a = vertices[edge_idx];
                let b = vertices[next_idx];
                masks.prims[prim.index()] = true;
                masks.vertices[a.index()] = true;
                masks.vertices[b.index()] = true;
                masks.points[geo.vertex_point(a).index()] = true;
                masks.points[geo.vertex_point(b).index()] = true;
            }
        }
        NormalGroupType::GuessFromGroup => unreachable!(),
    }

    Ok(masks)
}

fn write_computed_normal(
    geo: &mut Geometry,
    handle: &AttribHandle<[f32; 3]>,
    index: usize,
    normal: Vec3,
    keep_original_zero: bool,
) -> Result<(), SopError> {
    if normal.length_squared() < 1e-12 && keep_original_zero {
        return Ok(());
    }

    geo.set_attrib(handle, index, [normal.x, normal.y, normal.z])
        .map_err(SopError::Core)
}

fn apply_existing_normal_ops(
    geo: &mut Geometry,
    handle: &AttribHandle<[f32; 3]>,
    mask: &[bool],
    make_unit_length: bool,
    reverse_normals: bool,
) -> Result<(), SopError> {
    for (index, &selected) in mask.iter().enumerate() {
        if !selected {
            continue;
        }

        let mut normal = Vec3::from(geo.get_attrib(handle, index).map_err(SopError::Core)?);
        if make_unit_length {
            normal = normal.normalize_or_zero();
        }
        if reverse_normals {
            normal = -normal;
        }
        geo.set_attrib(handle, index, [normal.x, normal.y, normal.z])
            .map_err(SopError::Core)?;
    }

    Ok(())
}

impl Sop for NormalSop {
    type Params = NormalParams;

    fn name(&self) -> &'static str {
        "normal"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.override_normal.is_empty() {
            return Err(SopError::InvalidParam(
                "override_normal must not be empty".to_string(),
            ));
        }

        let mut geo = inputs[0].clone();
        let target_class = params.add_normals_to.attrib_class();
        let attr_exists = normal_attr_exists(&geo, target_class, &params.override_normal)?;

        let group_selection = if params.group.is_empty() {
            None
        } else {
            Some(selection_from_group(
                &geo,
                &params.group,
                params.group_type,
            )?)
        };

        let ignore_group_for_compute = params.compute_normals
            && !attr_exists
            && params.add_normals_to != NormalTarget::Vertices;
        let target_mask = if ignore_group_for_compute {
            SelectionMasks::all(&geo).for_target(params.add_normals_to, &geo)
        } else {
            group_selection
                .as_ref()
                .map(|selection| selection.for_target(params.add_normals_to, &geo))
                .unwrap_or_else(|| {
                    SelectionMasks::all(&geo).for_target(params.add_normals_to, &geo)
                })
        };
        let detail_prim_mask = if ignore_group_for_compute {
            vec![true; geo.num_prims()]
        } else {
            group_selection
                .as_ref()
                .map(|selection| selection.prims.clone())
                .unwrap_or_else(|| vec![true; geo.num_prims()])
        };

        if !params.compute_normals {
            if !attr_exists {
                return Ok(geo);
            }

            let handle = geo
                .find_attrib::<[f32; 3]>(target_class, &params.override_normal)
                .map_err(SopError::Core)?;
            apply_existing_normal_ops(
                &mut geo,
                &handle,
                &target_mask,
                params.make_unit_length,
                params.reverse_normals,
            )?;
            return Ok(geo);
        }

        let handle = ensure_normal_attrib(&mut geo, target_class, &params.override_normal)?;
        let ctx = build_normal_context(&geo);

        if params.add_normals_to == NormalTarget::Vertices
            && !attr_exists
            && group_selection.is_some()
        {
            let smooth_normals = compute_vertex_normals(&ctx, params.weighting_method, 180.0);
            let all_vertices = vec![true; geo.num_vertices()];
            for (index, normal) in smooth_normals.iter().enumerate() {
                if index < all_vertices.len() {
                    write_computed_normal(
                        &mut geo,
                        &handle,
                        index,
                        *normal,
                        params.keep_original_zero,
                    )?;
                }
            }
        }

        match params.add_normals_to {
            NormalTarget::Points => {
                let normals = compute_point_normals(&ctx, params.weighting_method);
                for (index, normal) in normals.iter().enumerate() {
                    if index < target_mask.len() && target_mask[index] {
                        write_computed_normal(
                            &mut geo,
                            &handle,
                            index,
                            *normal,
                            params.keep_original_zero,
                        )?;
                    }
                }
            }
            NormalTarget::Vertices => {
                let normals =
                    compute_vertex_normals(&ctx, params.weighting_method, params.cusp_angle);
                for (index, normal) in normals.iter().enumerate() {
                    if index < target_mask.len() && target_mask[index] {
                        write_computed_normal(
                            &mut geo,
                            &handle,
                            index,
                            *normal,
                            params.keep_original_zero,
                        )?;
                    }
                }
            }
            NormalTarget::Primitives => {
                for (index, normal) in ctx.prim_normals.iter().enumerate() {
                    if index < target_mask.len() && target_mask[index] {
                        write_computed_normal(
                            &mut geo,
                            &handle,
                            index,
                            *normal,
                            params.keep_original_zero,
                        )?;
                    }
                }
            }
            NormalTarget::Detail => {
                let normal = compute_detail_normal(&ctx, &detail_prim_mask);
                write_computed_normal(&mut geo, &handle, 0, normal, params.keep_original_zero)?;
            }
        }

        if params.reverse_normals {
            apply_existing_normal_ops(&mut geo, &handle, &target_mask, false, true)?;
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::creation::grid::{GridOrientation, GridParams, GridSop};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn point_normals_on_grid_default() {
        let grid = generate(
            &GridSop,
            &GridParams {
                size: [2.0, 2.0],
                rows: 3,
                cols: 3,
                center: Vec3::ZERO,
                orientation: GridOrientation::XZ,
            },
        )
        .unwrap();

        let result = grid.apply(&NormalSop, &NormalParams::default()).unwrap();
        let n_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();

        let center_n = Vec3::from(result.get_attrib(&n_handle, 4).unwrap());
        assert_relative_eq!(center_n.x.abs(), 0.0, epsilon = 1e-4);
        assert_relative_eq!(center_n.z.abs(), 0.0, epsilon = 1e-4);
        assert_relative_eq!(center_n.y.abs(), 1.0, epsilon = 1e-4);
    }

    #[test]
    fn vertex_normals_respect_cusp_angle() {
        let box_geo = make_box();
        let result = box_geo
            .apply(
                &NormalSop,
                &NormalParams {
                    add_normals_to: NormalTarget::Vertices,
                    cusp_angle: 30.0,
                    ..Default::default()
                },
            )
            .unwrap();

        let prim_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Primitive, "prim_n")
            .err();
        assert!(prim_handle.is_some());

        let prim_normals = result
            .clone()
            .apply(
                &NormalSop,
                &NormalParams {
                    add_normals_to: NormalTarget::Primitives,
                    override_normal: "prim_n".to_string(),
                    ..Default::default()
                },
            )
            .unwrap();
        let prim_handle = prim_normals
            .find_attrib::<[f32; 3]>(AttribClass::Primitive, "prim_n")
            .unwrap();
        let vertex_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Vertex, "N")
            .unwrap();

        for prim_idx in 0..result.num_prims() {
            let prim_normal = Vec3::from(prim_normals.get_attrib(&prim_handle, prim_idx).unwrap());
            let prim = PrimHandle::from_index(prim_idx);
            for &vertex in result.prim_vertices(prim) {
                let vertex_normal =
                    Vec3::from(result.get_attrib(&vertex_handle, vertex.index()).unwrap());
                assert!(
                    prim_normal.dot(vertex_normal) > 0.999,
                    "vertex normal should match face normal when cusp is sharp"
                );
            }
        }
    }

    #[test]
    fn primitive_normals_point_outward() {
        let box_geo = make_box();
        let result = box_geo
            .apply(
                &NormalSop,
                &NormalParams {
                    add_normals_to: NormalTarget::Primitives,
                    ..Default::default()
                },
            )
            .unwrap();
        let n_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Primitive, "N")
            .unwrap();

        for prim_idx in 0..result.num_prims() {
            let prim = PrimHandle::from_index(prim_idx);
            let points = result.prim_points(prim);
            let centroid = points
                .iter()
                .map(|&point| result.point_pos(point))
                .sum::<Vec3>()
                / points.len() as f32;
            let normal = Vec3::from(result.get_attrib(&n_handle, prim_idx).unwrap());
            assert_relative_eq!(normal.length(), 1.0, epsilon = 1e-5);
            assert!(
                normal.dot(centroid) > 0.0,
                "primitive normal should point outward"
            );
        }
    }

    #[test]
    fn detail_normal_is_area_weighted_average() {
        let grid = generate(
            &GridSop,
            &GridParams {
                size: [2.0, 2.0],
                rows: 3,
                cols: 3,
                center: Vec3::ZERO,
                orientation: GridOrientation::XZ,
            },
        )
        .unwrap();

        let result = grid
            .apply(
                &NormalSop,
                &NormalParams {
                    add_normals_to: NormalTarget::Detail,
                    ..Default::default()
                },
            )
            .unwrap();
        let n_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Detail, "N")
            .unwrap();
        let normal = Vec3::from(result.get_attrib(&n_handle, 0).unwrap());
        assert_relative_eq!(normal.x.abs(), 0.0, epsilon = 1e-4);
        assert_relative_eq!(normal.z.abs(), 0.0, epsilon = 1e-4);
        assert_relative_eq!(normal.y.abs(), 1.0, epsilon = 1e-4);
    }

    #[test]
    fn override_normal_name_is_supported() {
        let box_geo = make_box();
        let result = box_geo
            .apply(
                &NormalSop,
                &NormalParams {
                    add_normals_to: NormalTarget::Primitives,
                    override_normal: "myN".to_string(),
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(
            result
                .find_attrib::<[f32; 3]>(AttribClass::Primitive, "myN")
                .is_ok()
        );
        assert!(
            result
                .find_attrib::<[f32; 3]>(AttribClass::Primitive, "N")
                .is_err()
        );
    }

    #[test]
    fn point_group_restricts_modified_points_when_attribute_exists() {
        let mut box_geo = make_box();
        box_geo
            .add_attrib(
                AttribClass::Point,
                "N",
                AttribDefault::Vector3([1.0, 0.0, 0.0]),
                TypeQualifier::Normal,
            )
            .unwrap();
        box_geo.create_point_group("first");
        box_geo
            .groups_mut()
            .point_group_mut("first")
            .unwrap()
            .add(0);

        let result = box_geo
            .apply(
                &NormalSop,
                &NormalParams {
                    group: "first".to_string(),
                    group_type: NormalGroupType::Points,
                    ..Default::default()
                },
            )
            .unwrap();
        let n_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();

        let selected = Vec3::from(result.get_attrib(&n_handle, 0).unwrap());
        let untouched = Vec3::from(result.get_attrib(&n_handle, 1).unwrap());
        assert_ne!(selected, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(untouched, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn vertex_group_prefills_smooth_normals_when_missing_attribute() {
        let mut box_geo = make_box();
        let num_vertices = box_geo.num_vertices();
        box_geo
            .groups_mut()
            .create_vertex_group("single_vertex", num_vertices);
        box_geo
            .groups_mut()
            .vertex_group_mut("single_vertex")
            .unwrap()
            .add(0);

        let result = box_geo
            .apply(
                &NormalSop,
                &NormalParams {
                    group: "single_vertex".to_string(),
                    group_type: NormalGroupType::Vertices,
                    add_normals_to: NormalTarget::Vertices,
                    cusp_angle: 0.0,
                    ..Default::default()
                },
            )
            .unwrap();
        let n_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Vertex, "N")
            .unwrap();

        let untouched = Vec3::from(result.get_attrib(&n_handle, 1).unwrap());
        assert!(
            untouched.length() > 0.9,
            "unselected vertices should still receive the smooth prefill"
        );
    }

    #[test]
    fn can_normalize_and_reverse_existing_normals_without_recomputing() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::ZERO);
        geo.add_attrib(
            AttribClass::Point,
            "N",
            AttribDefault::Vector3([2.0, 0.0, 0.0]),
            TypeQualifier::Normal,
        )
        .unwrap();

        let result = geo
            .apply(
                &NormalSop,
                &NormalParams {
                    compute_normals: false,
                    make_unit_length: true,
                    reverse_normals: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let n_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();
        let normal = Vec3::from(result.get_attrib(&n_handle, 0).unwrap());
        assert_relative_eq!(normal.x, -1.0, epsilon = 1e-5);
        assert_relative_eq!(normal.length(), 1.0, epsilon = 1e-5);
    }

    #[test]
    fn keep_original_zero_preserves_existing_value() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::ZERO);
        geo.add_attrib(
            AttribClass::Point,
            "N",
            AttribDefault::Vector3([1.0, 0.0, 0.0]),
            TypeQualifier::Normal,
        )
        .unwrap();

        let result = geo
            .apply(
                &NormalSop,
                &NormalParams {
                    keep_original_zero: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let n_handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();
        let normal = Vec3::from(result.get_attrib(&n_handle, 0).unwrap());
        assert_eq!(normal, Vec3::new(1.0, 0.0, 0.0));
    }
}
