"""Generate benchmark report from JSON results.

Reads individual JSON files from --input-dir, merges them,
and produces REPORT.md and report.html in --output-dir.
"""

import argparse
import json
import math
import os
import platform
import subprocess
from collections import defaultdict
from datetime import datetime, timezone


def load_results(input_dir: str) -> list[dict]:
    """Load all JSON result files and merge into one list."""
    results = []
    for fname in sorted(os.listdir(input_dir)):
        if not fname.endswith(".json"):
            continue
        path = os.path.join(input_dir, fname)
        with open(path) as f:
            content = f.read().strip()
            if not content:
                continue
            # Support both JSON array and JSON-lines format
            if content.startswith("["):
                results.extend(json.loads(content))
            else:
                for line in content.splitlines():
                    line = line.strip()
                    if line and line.startswith("{"):
                        try:
                            results.append(json.loads(line))
                        except json.JSONDecodeError:
                            pass
    return results


def get_system_info() -> dict:
    """Gather system information."""
    info = {
        "os": platform.system(),
        "os_version": platform.release(),
        "arch": platform.machine(),
        "python": platform.python_version(),
    }
    try:
        result = subprocess.run(
            ["sysctl", "-n", "machdep.cpu.brand_string"],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            info["cpu"] = result.stdout.strip()
    except (FileNotFoundError, subprocess.TimeoutExpired):
        info["cpu"] = platform.processor() or "unknown"

    try:
        result = subprocess.run(
            ["sysctl", "-n", "hw.memsize"],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            info["ram_gb"] = round(int(result.stdout.strip()) / (1024**3), 1)
    except (FileNotFoundError, subprocess.TimeoutExpired, ValueError):
        pass

    return info


def format_ms(val: float | None) -> str:
    """Format milliseconds for display."""
    if val is None or val != val:  # None or NaN
        return "N/A"
    if val < 0.001:
        return f"{val * 1000:.1f}us"
    if val < 1.0:
        return f"{val:.3f}ms"
    if val < 1000.0:
        return f"{val:.2f}ms"
    return f"{val / 1000:.2f}s"


def framework_label(fw: str, lang: str) -> str:
    """Human-readable label for a framework."""
    labels = {
        ("procgeo", "rust"): "procgeo (Rust)",
        ("procgeo", "typescript"): "procgeo (Node.js)",
        ("procgeo", "python"): "procgeo (Python)",
        ("parry3d", "rust"): "parry3d (Rust)",
        ("meshopt", "rust"): "meshopt (Rust)",
        ("three.js", "typescript"): "three.js",
        ("blender_bpy", "python"): "Blender bpy",
        ("trimesh", "python"): "trimesh (Python)",
        ("pymeshlab", "python"): "PyMeshLab",
        ("open3d", "python"): "Open3D",
    }
    return labels.get((fw, lang), f"{fw} ({lang})")


# Framework sort order (lower = higher in table)
FW_ORDER = {
    ("procgeo", "rust"): 0,
    ("procgeo", "typescript"): 1,
    ("procgeo", "python"): 2,
    ("parry3d", "rust"): 3,
    ("meshopt", "rust"): 4,
    ("three.js", "typescript"): 5,
    ("blender_bpy", "python"): 6,
    ("trimesh", "python"): 7,
    ("pymeshlab", "python"): 8,
    ("open3d", "python"): 9,
}

CATEGORY_NAMES = {
    "creation": "Creation",
    "transform": "Transform",
    "topology": "Topology",
    "pipeline": "Full Pipeline",
}

SCALES = [100, 10_000, 100_000]


def build_tables(results: list[dict]) -> dict:
    """Organize results into nested dict: category -> operation -> (fw, lang) -> scale -> result."""
    tables = defaultdict(lambda: defaultdict(lambda: defaultdict(dict)))
    for r in results:
        # Skip malformed entries
        if not all(k in r for k in ("framework", "language", "category", "operation", "scale")):
            continue
        key = (r["framework"], r["language"])
        tables[r["category"]][r["operation"]][key][r["scale"]] = r
    return tables


def generate_markdown(results: list[dict], system_info: dict) -> str:
    """Generate the Markdown report."""
    tables = build_tables(results)
    lines = []

    lines.append("# ProcGeo Cross-Framework Benchmark Report")
    lines.append("")
    lines.append(f"**Generated:** {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}")
    lines.append("")

    # System info
    lines.append("## System")
    lines.append("")
    lines.append(f"- **OS:** {system_info.get('os', '?')} {system_info.get('os_version', '')}")
    lines.append(f"- **CPU:** {system_info.get('cpu', '?')}")
    if "ram_gb" in system_info:
        lines.append(f"- **RAM:** {system_info['ram_gb']} GB")
    lines.append(f"- **Arch:** {system_info.get('arch', '?')}")
    lines.append("")

    # Summary: find fastest per operation at 100K
    lines.append("## Summary")
    lines.append("")
    lines.append("Fastest framework per operation at 100K scale:")
    lines.append("")
    lines.append("| Operation | Fastest | Time |")
    lines.append("|-----------|---------|------|")

    for cat_key in ["creation", "transform", "topology", "pipeline"]:
        if cat_key not in tables:
            continue
        for op, fw_data in sorted(tables[cat_key].items()):
            best_fw = None
            best_time = float("inf")
            for (fw, lang), scale_data in fw_data.items():
                if 100_000 in scale_data:
                    t = scale_data[100_000].get("mean_ms")
                    if t is not None and t == t and t < best_time:  # not None/NaN
                        best_time = t
                        best_fw = framework_label(fw, lang)
                elif 10_000 in scale_data:
                    t = scale_data[10_000].get("mean_ms")
                    if t is not None and t == t and t < best_time:
                        best_time = t
                        best_fw = framework_label(fw, lang)
            if best_fw:
                lines.append(f"| {op} | **{best_fw}** | {format_ms(best_time)} |")

    lines.append("")

    # Binding overhead section
    lines.append("## Binding Overhead")
    lines.append("")
    lines.append("procgeo binding overhead vs native Rust (at 100K scale):")
    lines.append("")
    lines.append("| Operation | Rust | Node.js | Python | Node.js overhead | Python overhead |")
    lines.append("|-----------|------|---------|--------|-----------------|----------------|")

    for cat_key in ["creation", "transform", "topology", "pipeline"]:
        if cat_key not in tables:
            continue
        for op, fw_data in sorted(tables[cat_key].items()):
            rust_time = None
            node_time = None
            py_time = None
            for (fw, lang), scale_data in fw_data.items():
                target_scale = 100_000 if 100_000 in scale_data else (10_000 if 10_000 in scale_data else None)
                if target_scale is None:
                    continue
                t = scale_data[target_scale].get("mean_ms")
                if t is None or t != t:
                    continue
                if fw == "procgeo" and lang == "rust":
                    rust_time = t
                elif fw == "procgeo" and lang == "typescript":
                    node_time = t
                elif fw == "procgeo" and lang == "python":
                    py_time = t

            if rust_time is not None:
                node_oh = f"{node_time / rust_time:.1f}x" if node_time else "N/A"
                py_oh = f"{py_time / rust_time:.1f}x" if py_time else "N/A"
                lines.append(
                    f"| {op} | {format_ms(rust_time)} | "
                    f"{format_ms(node_time) if node_time else 'N/A'} | "
                    f"{format_ms(py_time) if py_time else 'N/A'} | "
                    f"{node_oh} | {py_oh} |"
                )

    lines.append("")

    # Detailed tables per category/operation
    for cat_key in ["creation", "transform", "topology", "pipeline"]:
        if cat_key not in tables:
            continue
        cat_name = CATEGORY_NAMES.get(cat_key, cat_key)
        lines.append(f"## {cat_name}")
        lines.append("")

        for op, fw_data in sorted(tables[cat_key].items()):
            lines.append(f"### {op}")
            lines.append("")

            # Header
            lines.append("| Framework | 100 | 10K | 100K |")
            lines.append("|-----------|-----|-----|------|")

            # Sort frameworks
            sorted_fws = sorted(
                fw_data.keys(), key=lambda k: FW_ORDER.get(k, 99)
            )

            for (fw, lang) in sorted_fws:
                scale_data = fw_data[(fw, lang)]
                label = framework_label(fw, lang)
                cells = []
                for s in SCALES:
                    if s in scale_data:
                        r = scale_data[s]
                        t = r.get("mean_ms")
                        cells.append(format_ms(t))
                    else:
                        cells.append("N/A")
                lines.append(f"| {label} | {' | '.join(cells)} |")

            lines.append("")

    return "\n".join(lines)


def generate_html(results: list[dict], system_info: dict) -> str:
    """Generate the HTML report."""
    tables = build_tables(results)

    # Collect all data for color scaling
    def color_for_value(val: float, best: float, worst: float) -> str:
        if val != val or best == worst:
            return "#f0f0f0"
        # Logarithmic scale for better visual spread
        if best <= 0 or worst <= 0 or val <= 0:
            return "#f0f0f0"
        log_val = math.log10(val)
        log_best = math.log10(best)
        log_worst = math.log10(worst)
        if log_worst == log_best:
            return "#22c55e"
        ratio = (log_val - log_best) / (log_worst - log_best)
        ratio = max(0, min(1, ratio))
        # Green (best) -> Yellow -> Red (worst)
        if ratio < 0.5:
            r = int(34 + (234 - 34) * (ratio * 2))
            g = int(197 + (179 - 197) * (ratio * 2))
            b = int(94 + (8 - 94) * (ratio * 2))
        else:
            r2 = (ratio - 0.5) * 2
            r = int(234 + (239 - 234) * r2)
            g = int(179 + (68 - 179) * r2)
            b = int(8 + (68 - 8) * r2)
        return f"#{r:02x}{g:02x}{b:02x}"

    html_parts = []
    html_parts.append("""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>ProcGeo Benchmark Report</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #0f172a; color: #e2e8f0; padding: 2rem; line-height: 1.6;
  }
  h1 { font-size: 2rem; margin-bottom: 0.5rem; color: #f8fafc; }
  h2 { font-size: 1.5rem; margin: 2rem 0 1rem; color: #94a3b8; border-bottom: 1px solid #334155; padding-bottom: 0.5rem; }
  h3 { font-size: 1.1rem; margin: 1.5rem 0 0.75rem; color: #cbd5e1; }
  .meta { color: #64748b; margin-bottom: 2rem; font-size: 0.9rem; }
  .system-info { background: #1e293b; padding: 1rem 1.5rem; border-radius: 8px; margin-bottom: 2rem; }
  .system-info span { margin-right: 2rem; }
  .summary-grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem; margin-bottom: 2rem;
  }
  .summary-card {
    background: #1e293b; border-radius: 8px; padding: 1rem 1.5rem;
    border-left: 4px solid #22c55e;
  }
  .summary-card .op { font-size: 0.85rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; }
  .summary-card .fw { font-size: 1.1rem; font-weight: 600; color: #f8fafc; }
  .summary-card .time { font-size: 0.9rem; color: #22c55e; }
  table {
    width: 100%; border-collapse: collapse; margin-bottom: 1.5rem;
    background: #1e293b; border-radius: 8px; overflow: hidden;
  }
  th {
    background: #334155; padding: 0.75rem 1rem; text-align: left;
    font-weight: 600; color: #f8fafc; font-size: 0.85rem;
    text-transform: uppercase; letter-spacing: 0.05em;
  }
  td {
    padding: 0.6rem 1rem; border-top: 1px solid #334155;
    font-size: 0.9rem; font-variant-numeric: tabular-nums;
  }
  td.value { text-align: right; font-weight: 500; }
  td.na { color: #475569; text-align: right; }
  td.fw-name { font-weight: 500; }
  tr:hover td { background: rgba(255,255,255,0.03); }
  .fastest { font-weight: 700; }
  .section { margin-bottom: 3rem; }
  .overhead-table td.overhead { font-weight: 600; }
</style>
</head>
<body>
""")

    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    html_parts.append(f'<h1>ProcGeo Cross-Framework Benchmark Report</h1>')
    html_parts.append(f'<p class="meta">Generated: {timestamp}</p>')

    # System info
    html_parts.append('<div class="system-info">')
    html_parts.append(f'<span><strong>OS:</strong> {system_info.get("os", "?")} {system_info.get("os_version", "")}</span>')
    html_parts.append(f'<span><strong>CPU:</strong> {system_info.get("cpu", "?")}</span>')
    if "ram_gb" in system_info:
        html_parts.append(f'<span><strong>RAM:</strong> {system_info["ram_gb"]} GB</span>')
    html_parts.append(f'<span><strong>Arch:</strong> {system_info.get("arch", "?")}</span>')
    html_parts.append('</div>')

    # Summary cards
    html_parts.append('<h2>Fastest per Operation (100K scale)</h2>')
    html_parts.append('<div class="summary-grid">')
    for cat_key in ["creation", "transform", "topology", "pipeline"]:
        if cat_key not in tables:
            continue
        for op, fw_data in sorted(tables[cat_key].items()):
            best_fw = None
            best_time = float("inf")
            for (fw, lang), scale_data in fw_data.items():
                for s in [100_000, 10_000]:
                    if s in scale_data:
                        t = scale_data[s].get("mean_ms")
                        if t is not None and t == t and t < best_time:
                            best_time = t
                            best_fw = framework_label(fw, lang)
                        break
            if best_fw:
                html_parts.append(f'''<div class="summary-card">
  <div class="op">{cat_key} / {op}</div>
  <div class="fw">{best_fw}</div>
  <div class="time">{format_ms(best_time)}</div>
</div>''')
    html_parts.append('</div>')

    # Detailed tables
    for cat_key in ["creation", "transform", "topology", "pipeline"]:
        if cat_key not in tables:
            continue
        cat_name = CATEGORY_NAMES.get(cat_key, cat_key)
        html_parts.append(f'<div class="section">')
        html_parts.append(f'<h2>{cat_name}</h2>')

        for op, fw_data in sorted(tables[cat_key].items()):
            html_parts.append(f'<h3>{op}</h3>')
            html_parts.append('<table>')
            html_parts.append('<tr><th>Framework</th><th style="text-align:right">100</th><th style="text-align:right">10K</th><th style="text-align:right">100K</th></tr>')

            # Find min/max per scale for coloring
            scale_vals: dict[int, list[float]] = {s: [] for s in SCALES}
            for (fw, lang), scale_data in fw_data.items():
                for s in SCALES:
                    if s in scale_data:
                        v = scale_data[s].get("mean_ms")
                        if v is not None and v == v:  # not None/NaN
                            scale_vals[s].append(v)

            sorted_fws = sorted(fw_data.keys(), key=lambda k: FW_ORDER.get(k, 99))

            for (fw, lang) in sorted_fws:
                scale_data = fw_data[(fw, lang)]
                label = framework_label(fw, lang)
                html_parts.append(f'<tr><td class="fw-name">{label}</td>')

                for s in SCALES:
                    if s in scale_data:
                        v = scale_data[s].get("mean_ms")
                        if v is None or v != v:
                            html_parts.append('<td class="na" style="text-align:right">N/A</td>')
                        else:
                            vals = scale_vals[s]
                            best = min(vals) if vals else v
                            worst = max(vals) if vals else v
                            color = color_for_value(v, best, worst)
                            is_fastest = v == best and len(vals) > 1
                            cls = "value fastest" if is_fastest else "value"
                            html_parts.append(
                                f'<td class="{cls}" style="text-align:right;background:{color};color:#0f172a">'
                                f'{format_ms(v)}</td>'
                            )
                    else:
                        html_parts.append('<td class="na" style="text-align:right">N/A</td>')

                html_parts.append('</tr>')

            html_parts.append('</table>')

        html_parts.append('</div>')

    # Binding overhead table
    html_parts.append('<div class="section">')
    html_parts.append('<h2>Binding Overhead</h2>')
    html_parts.append('<table class="overhead-table">')
    html_parts.append('<tr><th>Operation</th><th style="text-align:right">Rust</th>'
                      '<th style="text-align:right">Node.js</th><th style="text-align:right">Python</th>'
                      '<th style="text-align:right">Node.js overhead</th><th style="text-align:right">Python overhead</th></tr>')

    for cat_key in ["creation", "transform", "topology", "pipeline"]:
        if cat_key not in tables:
            continue
        for op, fw_data in sorted(tables[cat_key].items()):
            rust_time = node_time = py_time = None
            for (fw, lang), scale_data in fw_data.items():
                for s in [100_000, 10_000]:
                    if s in scale_data:
                        t = scale_data[s].get("mean_ms")
                        if t is not None and t == t:
                            if fw == "procgeo" and lang == "rust":
                                rust_time = t
                            elif fw == "procgeo" and lang == "typescript":
                                node_time = t
                            elif fw == "procgeo" and lang == "python":
                                py_time = t
                        break

            if rust_time is not None:
                node_oh = f"{node_time / rust_time:.1f}x" if node_time else "N/A"
                py_oh = f"{py_time / rust_time:.1f}x" if py_time else "N/A"
                html_parts.append(
                    f'<tr><td>{op}</td>'
                    f'<td class="value" style="text-align:right">{format_ms(rust_time)}</td>'
                    f'<td class="value" style="text-align:right">{format_ms(node_time) if node_time else "N/A"}</td>'
                    f'<td class="value" style="text-align:right">{format_ms(py_time) if py_time else "N/A"}</td>'
                    f'<td class="overhead" style="text-align:right">{node_oh}</td>'
                    f'<td class="overhead" style="text-align:right">{py_oh}</td></tr>'
                )

    html_parts.append('</table>')
    html_parts.append('</div>')

    html_parts.append('</body></html>')
    return "\n".join(html_parts)


def main():
    parser = argparse.ArgumentParser(description="Generate benchmark report")
    parser.add_argument("--input-dir", required=True, help="Directory with JSON result files")
    parser.add_argument("--output-dir", required=True, help="Directory for output reports")
    args = parser.parse_args()

    results = load_results(args.input_dir)
    if not results:
        print("No benchmark results found!")
        return

    system_info = get_system_info()

    # Save merged results
    merged = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "system": system_info,
        "benchmarks": results,
    }
    os.makedirs(args.output_dir, exist_ok=True)
    with open(os.path.join(args.output_dir, "results.json"), "w") as f:
        json.dump(merged, f, indent=2)

    # Generate reports
    md = generate_markdown(results, system_info)
    with open(os.path.join(args.output_dir, "REPORT.md"), "w") as f:
        f.write(md)
    print(f"Wrote {os.path.join(args.output_dir, 'REPORT.md')}")

    html = generate_html(results, system_info)
    with open(os.path.join(args.output_dir, "report.html"), "w") as f:
        f.write(html)
    print(f"Wrote {os.path.join(args.output_dir, 'report.html')}")


if __name__ == "__main__":
    main()
