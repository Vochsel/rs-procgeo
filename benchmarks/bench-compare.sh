#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
PYTHON_DIR="$SCRIPT_DIR/python"
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
    (cd "$REPO_DIR/bindings/procgeo-node" && npm install --silent 2>/dev/null && npx napi build --platform --release) || echo "WARN: procgeo-node build failed, skipping Node.js procgeo benchmarks"
fi

echo ""
echo "=== Building procgeo-py binding ==="
PROCGEO_WHEEL=""
if [ -d "$REPO_DIR/bindings/procgeo-py" ]; then
    WHEEL_DIR="$REPO_DIR/bindings/procgeo-py/target/wheels"
    mkdir -p "$WHEEL_DIR"
    if (cd "$REPO_DIR/bindings/procgeo-py" && uvx maturin build --release --interpreter python3.13 --out "$WHEEL_DIR" 2>&1); then
        PROCGEO_WHEEL="$(ls -t "$WHEEL_DIR"/*.whl 2>/dev/null | head -1)"
        echo "Built wheel: $PROCGEO_WHEEL"
    else
        echo "WARN: procgeo-py build failed, skipping Python procgeo benchmarks"
    fi
fi

# ---------------------------------------------------------------------------
# Set up Python benchmark venv
# ---------------------------------------------------------------------------

echo ""
echo "=== Setting up Python benchmark environment ==="
BENCH_VENV="$PYTHON_DIR/.venv"
uv venv --quiet --python 3.13 "$BENCH_VENV"
source "$BENCH_VENV/bin/activate"

# Install dependencies
uv pip install --quiet numpy scipy

# Install procgeo wheel if available
if [ -n "$PROCGEO_WHEEL" ]; then
    uv pip install --quiet --force-reinstall "$PROCGEO_WHEEL"
    echo "Installed procgeo-py into benchmark venv"
fi

# Install other frameworks (continue on failure for each)
uv pip install --quiet trimesh 2>&1 || echo "WARN: could not install trimesh"
uv pip install --quiet pymeshlab 2>&1 || echo "WARN: could not install pymeshlab"
uv pip install --quiet open3d 2>&1 || echo "WARN: could not install open3d"
uv pip install --quiet bpy 2>&1 || echo "WARN: could not install bpy"

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
    (cd "$SCRIPT_DIR/typescript" && npm install --silent 2>/dev/null && npx tsx bench.ts 2>&1) > "$RESULTS_DIR/typescript.json" || echo "WARN: TypeScript benchmarks failed"
    echo "Done."
fi

# ---------------------------------------------------------------------------
# 4. Python benchmarks (all use the same venv)
# ---------------------------------------------------------------------------

echo ""
echo "=== Running Python benchmarks ==="

# procgeo-py
echo "  -> procgeo-py..."
(cd "$PYTHON_DIR" && python bench_procgeo.py) > "$RESULTS_DIR/python_procgeo.json" || echo "  WARN: procgeo-py benchmarks failed"

# Blender bpy
echo "  -> Blender bpy..."
(cd "$PYTHON_DIR" && python bench_blender.py) > "$RESULTS_DIR/python_blender.json" || echo "  WARN: Blender bpy benchmarks failed"

# trimesh
echo "  -> trimesh..."
(cd "$PYTHON_DIR" && python bench_trimesh.py) > "$RESULTS_DIR/python_trimesh.json" || echo "  WARN: trimesh benchmarks failed"

# PyMeshLab
echo "  -> PyMeshLab..."
(cd "$PYTHON_DIR" && python bench_pymeshlab.py) > "$RESULTS_DIR/python_pymeshlab.json" || echo "  WARN: PyMeshLab benchmarks failed"

# Open3D
echo "  -> Open3D..."
(cd "$PYTHON_DIR" && python bench_open3d.py) > "$RESULTS_DIR/python_open3d.json" || echo "  WARN: Open3D benchmarks failed"

deactivate 2>/dev/null || true

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
