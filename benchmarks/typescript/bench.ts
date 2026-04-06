import { performance } from "node:perf_hooks";

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
  // Warmup
  for (let i = 0; i < 3; i++) fn();

  // Probe
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

const SCALES = [100, 10_000, 100_000];

// ---------------------------------------------------------------------------
// procgeo-node benchmarks
// ---------------------------------------------------------------------------

async function benchProcgeoNode() {
  let pg: any;
  try {
    pg = await import("procgeo-node");
  } catch {
    console.error(
      '{"error": "procgeo-node not built. Run: cd bindings/procgeo-node && npm run build"}'
    );
    return;
  }

  const fw = "procgeo";
  const lang = "typescript";

  for (const scale of SCALES) {
    const rc = gridRC(scale);

    // Creation: Grid
    let r = bench(() => pg.createGrid({ rows: rc, cols: rc }));
    emit({ framework: fw, language: lang, category: "creation", operation: "grid", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Creation: Sphere
    const sr = Math.max(3, Math.round(rc * 0.7));
    const sc = Math.max(4, Math.round(rc * 1.4));
    r = bench(() => pg.createSphere({ rows: sr, cols: sc }));
    emit({ framework: fw, language: lang, category: "creation", operation: "sphere", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Creation: Box
    r = bench(() => pg.createBox({}));
    emit({ framework: fw, language: lang, category: "creation", operation: "box", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Transform
    const grid = pg.createGrid({ rows: rc, cols: rc });
    r = bench(() =>
      pg.transform(grid, { translate: [10, 0, 0], scale: [2, 2, 2] })
    );
    emit({ framework: fw, language: lang, category: "transform", operation: "translate_scale", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Subdivide (small scales)
    if (scale <= 10_000) {
      r = bench(() => pg.subdivide(grid, { depth: 1 }));
      emit({ framework: fw, language: lang, category: "transform", operation: "subdivide", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });
    }

    // Smooth
    r = bench(() => pg.smooth(grid, { iterations: 3, strength: 0.5 }));
    emit({ framework: fw, language: lang, category: "transform", operation: "smooth", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Fuse
    r = bench(() => pg.fuse(grid, { distance: 0.001 }));
    emit({ framework: fw, language: lang, category: "topology", operation: "fuse", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Scatter
    r = bench(() => pg.scatter(grid, { count: scale, seed: 42 }));
    emit({ framework: fw, language: lang, category: "topology", operation: "scatter", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Full Pipeline
    r = bench(() => {
      let g = pg.createGrid({ rows: rc, cols: rc });
      g = pg.transform(g, { translate: [0, 1, 0], scale: [2, 2, 2] });
      g = pg.smooth(g, { iterations: 2, strength: 0.5 });
      g = pg.fuse(g, { distance: 0.001 });
    });
    emit({ framework: fw, language: lang, category: "pipeline", operation: "full_pipeline", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });
  }
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

    // Creation: Grid (PlaneGeometry)
    let r = bench(
      () => new THREE.PlaneGeometry(10, 10, rc - 1, rc - 1)
    );
    emit({ framework: fw, language: lang, category: "creation", operation: "grid", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Creation: Sphere
    const seg = Math.max(4, Math.round(rc * 1.4));
    const rings = Math.max(3, Math.round(rc * 0.7));
    r = bench(() => new THREE.SphereGeometry(1, seg, rings));
    emit({ framework: fw, language: lang, category: "creation", operation: "sphere", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Creation: Box
    r = bench(() => new THREE.BoxGeometry(1, 1, 1));
    emit({ framework: fw, language: lang, category: "creation", operation: "box", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Transform (apply matrix to all positions)
    const plane = new THREE.PlaneGeometry(10, 10, rc - 1, rc - 1);
    const matrix = new THREE.Matrix4().compose(
      new THREE.Vector3(10, 0, 0),
      new THREE.Quaternion(),
      new THREE.Vector3(2, 2, 2)
    );
    r = bench(() => {
      const g = plane.clone();
      g.applyMatrix4(matrix);
    });
    emit({ framework: fw, language: lang, category: "transform", operation: "translate_scale", scale, mean_ms: r.mean, std_ms: r.std, iterations: r.iters });

    // Smooth — three.js has no built-in smooth, mark N/A
    emit({ framework: fw, language: lang, category: "transform", operation: "smooth", scale, mean_ms: NaN, std_ms: 0, iterations: 0 });

    // Fuse — no built-in, N/A
    emit({ framework: fw, language: lang, category: "topology", operation: "fuse", scale, mean_ms: NaN, std_ms: 0, iterations: 0 });

    // Scatter — no built-in, N/A
    emit({ framework: fw, language: lang, category: "topology", operation: "scatter", scale, mean_ms: NaN, std_ms: 0, iterations: 0 });

    // Full Pipeline (create + transform only since three.js lacks smooth/fuse)
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
  console.error("Running procgeo-node benchmarks...");
  await benchProcgeoNode();

  console.error("Running three.js benchmarks...");
  await benchThreeJS();
}

main().catch(console.error);
