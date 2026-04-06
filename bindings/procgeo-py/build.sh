#!/bin/bash
set -e
echo "Building procgeo Python bindings..."
cd "$(dirname "$0")"

# Check for maturin
if ! command -v maturin &> /dev/null; then
    echo "Installing maturin..."
    pip install maturin
fi

maturin develop --release
echo "Build complete! The procgeo module is now available in Python."
echo "Usage: import procgeo"
