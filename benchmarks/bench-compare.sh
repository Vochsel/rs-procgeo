#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
mkdir -p "$RESULTS_DIR"

# Clean previous results
rm -f "$RESULTS_DIR"/*.json "$RESULTS_DIR"/REPORT.md "$RESULTS_DIR"/report.html

echo "============================================"
echo " ProcGeo Cross-Framework Benchmark Suite"
echo "============================================"
echo ""

# ---------------------------------------------------------------------------
# 1. Build bindings
# ---------------------------------------------------------------------------

echo "=== Building procgeo-node binding ==="
if [ -d "$REPO_DIR/bindings/procgeo-node" ]; then
    (cd "$REPO_DIR/bindings/procgeo-node" && npm run build 2>&1) || echo "WARN: procgeo-node build failed, skipping Node.js procgeo benchmarks"
fi

echo ""
echo "=== Building procgeo-py binding ==="
if [ -d "$REPO_DIR/bindings/procgeo-py" ]; then
    (cd "$REPO_DIR/bindings/procgeo-py" && maturin develop --release 2>&1) || echo "WARN: procgeo-py build failed, skipping Python procgeo benchmarks"
fi

# ---------------------------------------------------------------------------
# 2. Rust benchmarks
# ---------------------------------------------------------------------------

echo ""
echo "=== Running Rust benchmarks ==="
(cd "$SCRIPT_DIR/rust" && cargo run --release 2>/dev/null) > "$RESULTS_DIR/rust.json"
echo "Done. $(wc -l < "$RESULTS_DIR/rust.json" | tr -d ' ') results."

# ---------------------------------------------------------------------------
# 3. TypeScript benchmarks
# ---------------------------------------------------------------------------

echo ""
echo "=== Running TypeScript benchmarks ==="
if [ -d "$SCRIPT_DIR/typescript" ]; then
    (cd "$SCRIPT_DIR/typescript" && npm install --silent 2>/dev/null && npx tsx bench.ts 2>/dev/null) > "$RESULTS_DIR/typescript.json" || echo "WARN: TypeScript benchmarks failed"
    echo "Done."
fi

# ---------------------------------------------------------------------------
# 4. Python benchmarks
# ---------------------------------------------------------------------------

echo ""
echo "=== Running Python benchmarks ==="

PYTHON_DIR="$SCRIPT_DIR/python"

# procgeo-py
echo "  -> procgeo-py..."
(cd "$PYTHON_DIR" && uv run python bench_procgeo.py 2>/dev/null) > "$RESULTS_DIR/python_procgeo.json" 2>/dev/null || echo "  WARN: procgeo-py benchmarks failed"

# Blender bpy
echo "  -> Blender bpy..."
(cd "$PYTHON_DIR" && uv run --extra blender python bench_blender.py 2>/dev/null) > "$RESULTS_DIR/python_blender.json" 2>/dev/null || echo "  WARN: Blender bpy benchmarks failed (bpy may not be installable on this system)"

# trimesh
echo "  -> trimesh..."
(cd "$PYTHON_DIR" && uv run python bench_trimesh.py 2>/dev/null) > "$RESULTS_DIR/python_trimesh.json" 2>/dev/null || echo "  WARN: trimesh benchmarks failed"

# PyMeshLab
echo "  -> PyMeshLab..."
(cd "$PYTHON_DIR" && uv run python bench_pymeshlab.py 2>/dev/null) > "$RESULTS_DIR/python_pymeshlab.json" 2>/dev/null || echo "  WARN: PyMeshLab benchmarks failed"

# Open3D
echo "  -> Open3D..."
(cd "$PYTHON_DIR" && uv run python bench_open3d.py 2>/dev/null) > "$RESULTS_DIR/python_open3d.json" 2>/dev/null || echo "  WARN: Open3D benchmarks failed"

echo ""

# ---------------------------------------------------------------------------
# 5. Generate reports
# ---------------------------------------------------------------------------

echo "=== Generating reports ==="
(cd "$SCRIPT_DIR/report-generator" && uv run python generate.py \
    --input-dir "$RESULTS_DIR" \
    --output-dir "$RESULTS_DIR")

echo ""
echo "============================================"
echo " Benchmark complete!"
echo "============================================"
echo ""
echo "Results:"
echo "  Markdown: $RESULTS_DIR/REPORT.md"
echo "  HTML:     $RESULTS_DIR/report.html"
echo "  Raw JSON: $RESULTS_DIR/results.json"
