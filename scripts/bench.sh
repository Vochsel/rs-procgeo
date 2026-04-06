#!/bin/bash
set -e

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

step() { echo -e "\n${BLUE}==> $1${NC}"; }
ok()   { echo -e "${GREEN}    ✓ $1${NC}"; }
warn() { echo -e "${YELLOW}    ! $1${NC}"; }

echo -e "${BLUE}"
echo "  ┌──────────────────────────────────┐"
echo "  │  ProcGeo — Benchmark Suite       │"
echo "  └──────────────────────────────────┘"
echo -e "${NC}"

# Parse args
CRATE=""
FILTER=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --core)    CRATE="procgeo-core"; shift ;;
        --sops)    CRATE="procgeo-sops"; shift ;;
        --filter)  FILTER="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: ./bench.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --core          Run only procgeo-core benchmarks"
            echo "  --sops          Run only procgeo-sops benchmarks"
            echo "  --filter NAME   Run only benchmarks matching NAME (e.g. 'points' or 'attribs')"
            echo "  -h, --help      Show this help"
            echo ""
            echo "Examples:"
            echo "  ./bench.sh                      # Run all benchmarks"
            echo "  ./bench.sh --core               # Run core benchmarks only"
            echo "  ./bench.sh --filter points      # Run benchmarks matching 'points'"
            echo "  ./bench.sh --core --filter bbox  # Core benchmarks matching 'bbox'"
            echo ""
            echo "Reports are generated in target/criterion/ (open index.html in a browser)."
            exit 0
            ;;
        *)
            echo "Unknown option: $1 (use --help for usage)"
            exit 1
            ;;
    esac
done

# Build bench command
CMD="cargo bench"
if [[ -n "$CRATE" ]]; then
    CMD="$CMD -p $CRATE"
fi
if [[ -n "$FILTER" ]]; then
    CMD="$CMD -- $FILTER"
fi

step "Running benchmarks"
[[ -n "$CRATE" ]] && echo "       Crate: $CRATE" || echo "       Crate: all"
[[ -n "$FILTER" ]] && echo "       Filter: $FILTER"

echo ""
$CMD

# Summary
echo ""
ok "Benchmarks complete"
echo ""
echo "  HTML reports: target/criterion/report/index.html"
echo ""
