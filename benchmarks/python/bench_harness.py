"""Shared benchmark harness for Python frameworks."""

import json
import time
import statistics
import sys


def bench(fn, warmup=3, min_iters=10, min_duration=2.0):
    """Run a benchmark function and return (mean_ms, std_ms, iterations)."""
    # Warmup
    for _ in range(warmup):
        fn()

    # Probe to determine iteration count
    start = time.perf_counter()
    fn()
    probe_s = time.perf_counter() - start

    if probe_s < 0.001:
        iters = 1000
    elif probe_s < 0.01:
        iters = 200
    elif probe_s < 0.1:
        iters = 50
    else:
        iters = max(min_iters, int(min_duration / probe_s))

    times = []
    for _ in range(iters):
        start = time.perf_counter()
        fn()
        elapsed = (time.perf_counter() - start) * 1000.0  # ms
        times.append(elapsed)

    mean = statistics.mean(times)
    std = statistics.stdev(times) if len(times) > 1 else 0.0
    return mean, std, iters


def emit_result(framework, language, category, operation, scale, mean_ms, std_ms, iterations):
    """Print a single benchmark result as JSON."""
    result = {
        "framework": framework,
        "language": language,
        "category": category,
        "operation": operation,
        "scale": scale,
        "mean_ms": round(mean_ms, 4),
        "std_ms": round(std_ms, 4),
        "iterations": iterations,
    }
    print(json.dumps(result))
    sys.stdout.flush()


def grid_rc(target):
    """Grid rows/cols to produce approximately target vertices."""
    import math
    return int(math.ceil(math.sqrt(target)))


SCALES = [100, 10_000, 100_000]
