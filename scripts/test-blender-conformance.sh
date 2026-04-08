#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

PYTHON_VERSION="${PROCGEO_BLENDER_PYTHON:-3.13}"
TEST_DIR="$PWD/tests/blender_conformance"

uv run \
  --python "$PYTHON_VERSION" \
  --no-project \
  --with ./bindings/procgeo-py \
  --with bpy \
  --with pytest \
  python -m pytest "$TEST_DIR" "$@"
