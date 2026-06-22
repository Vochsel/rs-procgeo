import fs from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath, pathToFileURL } from "node:url";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

interface BenchResult {
  framework: string;
  language: string;
  category: string;
  operation: string;
  scale: number;
  mean_ms: number;
  std_ms: number;
  iterations: number;
}

function bench(fn: () => void): { mean: number; std: number; iters: number } {
  for (let i = 0; i < 3; i++) fn();

  const probeStart = performance.now();
  fn();
  const probeMs = performance.now() - probeStart;

  let iters: number;
  if (probeMs < 1) iters = 1000;
  else if (probeMs < 10) iters = 200;
  else if (probeMs < 100) iters = 50;
  else iters = 10;

  const times: number[] = [];
  for (let i = 0; i < iters; i++) {
    const start = performance.now();
    fn();
    times.push(performance.now() - start);
  }

  const mean = times.reduce((a, b) => a + b, 0) / times.length;
  const variance =
    times.reduce((a, t) => a + (t - mean) ** 2, 0) / times.length;
  const std = Math.sqrt(variance);

  return { mean, std, iters };
}

function emit(r: BenchResult) {
  console.log(JSON.stringify(r));
}

function gridRC(target: number): number {
  return Math.ceil(Math.sqrt(target));
}

function dispose(value: unknown) {
  if (value && typeof value === "object" && "free" in value) {
    const candidate = value as { free?: () => void };
    if (typeof candidate.free === "function") {
      candidate.free();
    }
  }
}

function emitBench(
  framework: string,
  category: string,
  operation: string,
  scale: number,
  result: { mean: number; std: number; iters: number },
) {
  emit({
    framework,
    language: "typescript",
    category,
    operation,
    scale,
    mean_ms: result.mean,
    std_ms: result.std,
    iterations: result.iters,
  });
}

const SCALES = [100, 10_000, 100_000];
const THIS_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_DIR = path.resolve(THIS_DIR, "../..");

function firstExistingPath(candidates: string[]): string | null {
  return candidates.find((candidate) => existsSync(candidate)) ?? null;
}

async function loadProcgeoWasm() {
  const jsPath = firstExistingPath([
    path.join(REPO_DIR, "bindings/procgeo-wasm/pkg/procgeo_wasm.js"),
    path.join(REPO_DIR, "web/wasm/procgeo_wasm.js"),
    path.join(REPO_DIR, "web/pkg/procgeo_wasm.js"),
  ]);
  const wasmPath = firstExistingPath([
    path.join(REPO_DIR, "bindings/procgeo-wasm/pkg/procgeo_wasm_bg.wasm"),
    path.join(REPO_DIR, "web/wasm/procgeo_wasm_bg.wasm"),
    path.join(REPO_DIR, "web/pkg/procgeo_wasm_bg.wasm"),
  ]);

  if (!jsPath || !wasmPath) {
    console.error(
      '{"error": "procgeo-wasm not built. Run: pnpm build:wasm or cd bindings/procgeo-wasm && ./build.sh"}',
    );
    return null;
  }

  try {
    const pg = await import(pathToFileURL(jsPath).href);
    const wasmBytes = await fs.readFile(wasmPath);
    await pg.default({ module_or_path: wasmBytes });
    return pg;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(JSON.stringify({ error: `procgeo-wasm init failed: ${message}` }));
    return null;
  }
}

function runProcgeoBenchmarks(framework: string, pg: any) {
  for (const scale of SCALES) {
    const rc = gridRC(scale);

    let r = bench(() => {
      const geo = pg.createGrid({ rows: rc, cols: rc });
      dispose(geo);
    });
    emitBench(framework, "creation", "grid", scale, r);

    const sr = Math.max(3, Math.round(rc * 0.7));
    const sc = Math.max(4, Math.round(rc * 1.4));
    r = bench(() => {
      const geo = pg.createSphere({ rows: sr, cols: sc });
      dispose(geo);
    });
    emitBench(framework, "creation", "sphere", scale, r);

    r = bench(() => {
      const geo = pg.createBox({});
      dispose(geo);
    });
    emitBench(framework, "creation", "box", scale, r);

    const grid = pg.createGrid({ rows: rc, cols: rc });
    try {
      r = bench(() => {
        const geo = pg.transform(grid, {
          translate: [10, 0, 0],
          scale: [2, 2, 2],
        });
        dispose(geo);
      });
      emitBench(framework, "transform", "translate_scale", scale, r);

      if (scale <= 10_000) {
        r = bench(() => {
          const geo = pg.subdivide(grid, { depth: 1 });
          dispose(geo);
        });
        emitBench(framework, "transform", "subdivide", scale, r);
      }

      r = bench(() => {
        const geo = pg.smooth(grid, { iterations: 3, strength: 0.5 });
        dispose(geo);
      });
      emitBench(framework, "transform", "smooth", scale, r);

      r = bench(() => {
        const geo = pg.fuse(grid, { distance: 0.001 });
        dispose(geo);
      });
      emitBench(framework, "topology", "fuse", scale, r);

      r = bench(() => {
        const geo = pg.scatter(grid, { count: scale, seed: 42 });
        dispose(geo);
      });
      emitBench(framework, "topology", "scatter", scale, r);
    } finally {
      dispose(grid);
    }

    // -- Softbody (XPBD), 10 simulated frames --
    {
      const cloth = pg.createGrid({ rows: rc, cols: rc });
      r = bench(() => {
        const out = pg.softbody(cloth, { frame: 10 });
        dispose(out);
      });
      emitBench(framework, "simulation", "softbody", scale, r);
      dispose(cloth);
    }

    r = bench(() => {
      const input = pg.createGrid({ rows: rc, cols: rc });
      const moved = pg.transform(input, {
        translate: [0, 1, 0],
        scale: [2, 2, 2],
      });
      const smoothed = pg.smooth(moved, { iterations: 2, strength: 0.5 });
      const fused = pg.fuse(smoothed, { distance: 0.001 });

      dispose(fused);
      dispose(smoothed);
      dispose(moved);
      dispose(input);
    });
    emitBench(framework, "pipeline", "full_pipeline", scale, r);
  }
}

async function benchProcgeoWasm() {
  const pg = await loadProcgeoWasm();
  if (!pg) return;
  runProcgeoBenchmarks("procgeo", pg);
}

// ---------------------------------------------------------------------------
// three.js benchmarks
// ---------------------------------------------------------------------------

async function benchThreeJS() {
  let THREE: any;
  try {
    THREE = await import("three");
  } catch {
    console.error('{"error": "three not installed. Run: npm install three"}');
    return;
  }

  const fw = "three.js";
  const lang = "typescript";

  for (const scale of SCALES) {
    const rc = gridRC(scale);

    let r = bench(
      () => new THREE.PlaneGeometry(10, 10, rc - 1, rc - 1),
    );
    emit({ framework: fw, language: lang, category: "creation", operation: "grid", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    const seg = Math.max(4, Math.round(rc * 1.4));
    const rings = Math.max(3, Math.round(rc * 0.7));
    r = bench(() => new THREE.SphereGeometry(1, seg, rings));
    emit({ framework: fw, language: lang, category: "creation", operation: "sphere", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    r = bench(() => new THREE.BoxGeometry(1, 1, 1));
    emit({ framework: fw, language: lang, category: "creation", operation: "box", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    const plane = new THREE.PlaneGeometry(10, 10, rc - 1, rc - 1);
    const matrix = new THREE.Matrix4().compose(
      new THREE.Vector3(10, 0, 0),
      new THREE.Quaternion(),
      new THREE.Vector3(2, 2, 2),
    );
    r = bench(() => {
      const g = plane.clone();
      g.applyMatrix4(matrix);
    });
    emit({ framework: fw, language: lang, category: "transform", operation: "translate_scale", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    emit({ framework: fw, language: lang, category: "transform", operation: "smooth", scale, mean_ms: NaN, std_ms: 0, iterations: 0 });
    emit({ framework: fw, language: lang, category: "topology", operation: "fuse", scale, mean_ms: NaN, std_ms: 0, iterations: 0 });
    emit({ framework: fw, language: lang, category: "topology", operation: "scatter", scale, mean_ms: NaN, std_ms: 0, iterations: 0 });

    r = bench(() => {
      const g = new THREE.PlaneGeometry(10, 10, rc - 1, rc - 1);
      g.applyMatrix4(matrix);
      g.computeVertexNormals();
    });
    emit({ framework: fw, language: lang, category: "pipeline", operation: "full_pipeline", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  console.error("Running procgeo-wasm benchmarks...");
  await benchProcgeoWasm();

  console.error("Running three.js benchmarks...");
  await benchThreeJS();
}

main().catch(console.error);
