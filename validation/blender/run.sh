#!/usr/bin/env bash
#
# Blender Validation Suite for procgeo SOPs
#
# Validates that procgeo geometry output matches Blender's equivalent
# primitives point-for-point, face-for-face.
#
# Usage:
#   ./validation/blender/run.sh                        # run all tests
#   ./validation/blender/run.sh --filter box           # run only box tests
#   ./validation/blender/run.sh --blender /path/to/blender
#   ./validation/blender/run.sh --validate-only        # skip Blender export (use cached reference)
#   ./validation/blender/run.sh --export-only          # only generate Blender reference
#
# Prerequisites:
#   1. Blender 3.0+ installed and accessible via `blender` (or --blender flag)
#   2. procgeo Python bindings built:
#        cd bindings/procgeo-py && maturin develop --release
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BLENDER="${BLENDER:-blender}"
FILTER=""
VALIDATE_ONLY=false
EXPORT_ONLY=false
TOLERANCE="1e-4"

while [[ $# -gt 0 ]]; do
    case $1 in
        --blender)      BLENDER="$2"; shift 2 ;;
        --filter)       FILTER="$2"; shift 2 ;;
        --tolerance)    TOLERANCE="$2"; shift 2 ;;
        --validate-only) VALIDATE_ONLY=true; shift ;;
        --export-only)  EXPORT_ONLY=true; shift ;;
        -h|--help)
            sed -n '3,14p' "$0" | sed 's/^# \?//'
            exit 0 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo ""
echo "=== procgeo Blender Validation Suite ==="
echo ""

# --- Phase 1: Export reference geometry from Blender ---

if [[ "$VALIDATE_ONLY" == false ]]; then
    if ! command -v "$BLENDER" &>/dev/null; then
        echo "ERROR: Blender not found at '$BLENDER'"
        echo "       Install Blender or set --blender /path/to/blender"
        echo ""
        echo "       To skip this phase and use cached reference data:"
        echo "         $0 --validate-only"
        exit 1
    fi

    echo "Phase 1: Exporting reference geometry from Blender..."
    echo ""

    BLENDER_ARGS="--background --python $SCRIPT_DIR/blender_export.py --"
    BLENDER_ARGS="$BLENDER_ARGS --output $SCRIPT_DIR/reference"
    if [[ -n "$FILTER" ]]; then
        BLENDER_ARGS="$BLENDER_ARGS --filter $FILTER"
    fi

    # Run Blender, suppress its startup output but keep our prints
    "$BLENDER" $BLENDER_ARGS 2>&1 | grep -E "^  (exported|[0-9])" || true
    echo ""

    if [[ "$EXPORT_ONLY" == true ]]; then
        echo "Export complete. Reference files in: $SCRIPT_DIR/reference/"
        exit 0
    fi
fi

# --- Phase 2: Validate procgeo output ---

echo "Phase 2: Validating procgeo output against Blender reference..."
echo ""

VALIDATE_ARGS="--reference $SCRIPT_DIR/reference --tolerance $TOLERANCE"
if [[ -n "$FILTER" ]]; then
    VALIDATE_ARGS="$VALIDATE_ARGS --filter $FILTER"
fi

python3 "$SCRIPT_DIR/validate.py" $VALIDATE_ARGS
