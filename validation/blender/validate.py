"""Validate procgeo SOP output against Blender reference geometry.

Run via: python3 validate.py [--reference DIR] [--filter PATTERN] [--tolerance F]

Loads Blender-exported JSON reference files, creates equivalent geometry using
the procgeo Python bindings, and compares point-for-point, face-for-face.

Requires: procgeo Python package (build with `maturin develop` in bindings/procgeo-py/)
"""

import argparse
import json
import math
import os
import sys


# ---------------------------------------------------------------------------
# Geometry extraction from procgeo
# ---------------------------------------------------------------------------

def extract_procgeo(geo, include_normals=False):
    """Extract geometry data from a procgeo Geometry object."""
    positions = []
    for i in range(geo.num_points):
        p = geo.point_pos(i)
        positions.append([round(p[0], 6), round(p[1], 6), round(p[2], 6)])

    faces = []
    for i in range(geo.num_prims):
        faces.append(geo.prim_point_indices(i))

    face_vertex_counts = sorted(len(f) for f in faces)

    xs = [p[0] for p in positions] if positions else [0]
    ys = [p[1] for p in positions] if positions else [0]
    zs = [p[2] for p in positions] if positions else [0]

    data = {
        "num_points": geo.num_points,
        "num_faces": geo.num_prims,
        "positions": positions,
        "faces": faces,
        "face_vertex_counts": face_vertex_counts,
        "bbox_min": [min(xs), min(ys), min(zs)],
        "bbox_max": [max(xs), max(ys), max(zs)],
    }

    if include_normals:
        normals = []
        attrib_data = geo.attrib_data("point", "N")
        if attrib_data:
            # attrib_data returns flat [x,y,z,x,y,z,...] for Vec3 attributes
            for i in range(0, len(attrib_data), 3):
                normals.append([
                    round(attrib_data[i], 6),
                    round(attrib_data[i + 1], 6),
                    round(attrib_data[i + 2], 6),
                ])
        data["normals"] = normals

    return data


# ---------------------------------------------------------------------------
# Comparison engine
# ---------------------------------------------------------------------------

def compare_counts(ref, test):
    """Compare point and face counts. Returns list of error strings."""
    errors = []
    if ref["num_points"] != test["num_points"]:
        errors.append(
            f"point count: blender={ref['num_points']}, procgeo={test['num_points']}"
        )
    if ref["num_faces"] != test["num_faces"]:
        errors.append(
            f"face count: blender={ref['num_faces']}, procgeo={test['num_faces']}"
        )
    return errors


def compare_positions(ref, test, tolerance):
    """Compare sorted position arrays within tolerance."""
    errors = []
    if ref["num_points"] != test["num_points"]:
        return errors  # already reported by compare_counts

    ref_sorted = sorted(ref["positions"])
    test_sorted = sorted(test["positions"])

    mismatches = 0
    first_mismatch = None
    for i, (r, t) in enumerate(zip(ref_sorted, test_sorted)):
        dist = math.sqrt(sum((a - b) ** 2 for a, b in zip(r, t)))
        if dist > tolerance:
            mismatches += 1
            if first_mismatch is None:
                first_mismatch = (i, r, t, dist)

    if mismatches > 0:
        i, r, t, dist = first_mismatch
        errors.append(
            f"position mismatch: {mismatches}/{ref['num_points']} points differ "
            f"(first at sorted idx {i}: blender={r}, procgeo={t}, dist={dist:.6f})"
        )
    return errors


def compare_topology(ref, test):
    """Compare face topology (vertex count distribution)."""
    errors = []
    if ref["face_vertex_counts"] != test["face_vertex_counts"]:
        # Summarize the difference
        ref_summary = _count_summary(ref["face_vertex_counts"])
        test_summary = _count_summary(test["face_vertex_counts"])
        errors.append(
            f"face topology: blender={ref_summary}, procgeo={test_summary}"
        )
    return errors


def compare_faces_by_position(ref, test, tolerance):
    """Compare faces using position-based matching (handles different point ordering)."""
    errors = []
    if ref["num_points"] != test["num_points"] or ref["num_faces"] != test["num_faces"]:
        return errors  # can't compare if counts differ

    ref_norm = _normalize_faces(ref["positions"], ref["faces"])
    test_norm = _normalize_faces(test["positions"], test["faces"])

    mismatches = 0
    first_mismatch = None
    for i, (r, t) in enumerate(zip(ref_norm, test_norm)):
        if not _faces_match(r, t, tolerance):
            mismatches += 1
            if first_mismatch is None:
                first_mismatch = i

    if mismatches > 0:
        errors.append(
            f"face mismatch: {mismatches}/{ref['num_faces']} faces differ "
            f"(first at sorted idx {first_mismatch})"
        )
    return errors


def compare_bbox(ref, test, tolerance):
    """Compare bounding boxes."""
    errors = []
    for label, rk, tk in [("min", "bbox_min", "bbox_min"), ("max", "bbox_max", "bbox_max")]:
        for axis, rv, tv in zip("xyz", ref[rk], test[tk]):
            if abs(rv - tv) > tolerance:
                errors.append(
                    f"bbox {label}: blender={ref[rk]}, procgeo={test[tk]}"
                )
                break  # one message per min/max
        else:
            continue
        break
    return errors


def compare_normals(ref, test, tolerance):
    """Compare vertex normals if both sides have them."""
    errors = []
    ref_normals = ref.get("normals", [])
    test_normals = test.get("normals", [])

    if not ref_normals or not test_normals:
        return errors

    if len(ref_normals) != len(test_normals):
        errors.append(
            f"normal count: blender={len(ref_normals)}, procgeo={len(test_normals)}"
        )
        return errors

    # Sort normals for comparison (point ordering may differ)
    ref_sorted = sorted(ref_normals)
    test_sorted = sorted(test_normals)

    mismatches = 0
    for r, t in zip(ref_sorted, test_sorted):
        dist = math.sqrt(sum((a - b) ** 2 for a, b in zip(r, t)))
        if dist > tolerance:
            mismatches += 1

    if mismatches > 0:
        errors.append(
            f"normal mismatch: {mismatches}/{len(ref_normals)} normals differ (tol={tolerance})"
        )
    return errors


def _count_summary(counts):
    """Summarize a list of face vertex counts, e.g. {3: 12, 4: 6}."""
    summary = {}
    for c in counts:
        summary[c] = summary.get(c, 0) + 1
    return summary


def _normalize_faces(positions, faces):
    """Convert index-based faces to position-based, then sort for comparison.

    Each face becomes a tuple of sorted vertex positions, and the full list
    of faces is sorted lexicographically.
    """
    result = []
    for face in faces:
        face_positions = tuple(
            sorted(tuple(positions[i]) for i in face)
        )
        result.append(face_positions)
    return sorted(result)


def _faces_match(face_a, face_b, tolerance):
    """Check if two position-based faces match within tolerance."""
    if len(face_a) != len(face_b):
        return False
    for pa, pb in zip(face_a, face_b):
        dist = math.sqrt(sum((a - b) ** 2 for a, b in zip(pa, pb)))
        if dist > tolerance:
            return False
    return True


# ---------------------------------------------------------------------------
# Test definitions — must match blender_export.py test names
# ---------------------------------------------------------------------------

def make_tests(pg):
    """Build test dict using the procgeo module. Each returns extracted geo data."""

    def test_box_default():
        return extract_procgeo(pg.create_box())

    def test_box_scaled():
        return extract_procgeo(pg.create_box(size_x=2.0, size_y=3.0, size_z=4.0))

    def test_box_offset():
        return extract_procgeo(pg.create_box(center_x=1.0, center_y=2.0, center_z=3.0))

    def test_grid_default():
        return extract_procgeo(pg.create_grid(rows=10, cols=10, size_x=10.0, size_y=10.0))

    def test_sphere_default():
        return extract_procgeo(pg.create_sphere(radius=0.5, rows=12, cols=24))

    def test_circle_default():
        return extract_procgeo(pg.create_circle(radius=1.0, divisions=40))

    def test_tube_default():
        return extract_procgeo(pg.create_tube())

    def test_torus_default():
        return extract_procgeo(pg.create_torus(
            radius_outer=1.0, radius_inner=0.3, rows=12, cols=24,
        ))

    def test_transform_translate():
        geo = pg.create_box()
        geo = pg.transform(geo, translate_x=1.0, translate_y=2.0, translate_z=3.0)
        return extract_procgeo(geo)

    def test_transform_uniform_scale():
        geo = pg.create_box()
        geo = pg.transform(geo, scale_x=2.0, scale_y=2.0, scale_z=2.0)
        return extract_procgeo(geo)

    def test_transform_rotate_z():
        geo = pg.create_box()
        geo = pg.transform(geo, rotate_y=45.0)  # 45° around Y (up) axis
        return extract_procgeo(geo)

    def test_normals_box():
        geo = pg.create_box()
        geo = pg.compute_normals(geo)
        return extract_procgeo(geo, include_normals=True)

    return {
        "box_default": test_box_default,
        "box_scaled": test_box_scaled,
        "box_offset": test_box_offset,
        "grid_default": test_grid_default,
        "sphere_default": test_sphere_default,
        "circle_default": test_circle_default,
        "tube_default": test_tube_default,
        "torus_default": test_torus_default,
        "transform_translate": test_transform_translate,
        "transform_uniform_scale": test_transform_uniform_scale,
        "transform_rotate_z": test_transform_rotate_z,
        "normals_box": test_normals_box,
    }


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def run_validation(reference_dir, filter_pat=None, tolerance=1e-4, normal_tolerance=0.05):
    try:
        import procgeo
    except ImportError:
        print("ERROR: Could not import procgeo Python bindings.")
        print("       Build them first:")
        print("         cd bindings/procgeo-py && maturin develop --release")
        return 1

    tests = make_tests(procgeo)
    passed = 0
    failed = 0
    skipped = 0

    for name in sorted(tests):
        if filter_pat and filter_pat not in name:
            continue

        ref_path = os.path.join(reference_dir, f"{name}.json")
        if not os.path.exists(ref_path):
            print(f"  SKIP  {name:35s}  (no reference file)")
            skipped += 1
            continue

        with open(ref_path) as f:
            ref = json.load(f)

        try:
            test_data = tests[name]()
        except Exception as e:
            print(f"  FAIL  {name:35s}  (procgeo error: {e})")
            failed += 1
            continue

        # Run all comparisons
        errors = []
        errors.extend(compare_counts(ref, test_data))
        errors.extend(compare_bbox(ref, test_data, tolerance))
        errors.extend(compare_positions(ref, test_data, tolerance))
        errors.extend(compare_topology(ref, test_data))
        errors.extend(compare_faces_by_position(ref, test_data, tolerance))

        # Normals — only for tests that include them, use looser tolerance
        if "normals" in name:
            errors.extend(compare_normals(ref, test_data, normal_tolerance))

        if errors:
            print(f"  FAIL  {name}")
            for e in errors:
                print(f"        {e}")
            failed += 1
        else:
            pts = ref["num_points"]
            faces = ref["num_faces"]
            print(f"  PASS  {name:35s}  ({pts} pts, {faces} faces)")
            passed += 1

    print()
    total = passed + failed
    if total == 0:
        print("  No tests matched.")
        return 0

    status = "PASS" if failed == 0 else "FAIL"
    print(f"  {status}: {passed} passed, {failed} failed", end="")
    if skipped:
        print(f", {skipped} skipped", end="")
    print(f" ({total} total)")

    return 1 if failed else 0


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Validate procgeo output against Blender reference geometry."
    )
    parser.add_argument(
        "--reference", default=os.path.join(os.path.dirname(__file__), "reference"),
        help="Directory containing Blender reference JSON files",
    )
    parser.add_argument(
        "--filter", default=None,
        help="Only run tests whose name contains this substring",
    )
    parser.add_argument(
        "--tolerance", type=float, default=1e-4,
        help="Position comparison tolerance (default: 1e-4)",
    )
    parser.add_argument(
        "--normal-tolerance", type=float, default=0.05,
        help="Normal comparison tolerance (default: 0.05)",
    )
    args = parser.parse_args()

    print()
    rc = run_validation(
        args.reference,
        filter_pat=args.filter,
        tolerance=args.tolerance,
        normal_tolerance=args.normal_tolerance,
    )
    sys.exit(rc)


if __name__ == "__main__":
    main()
