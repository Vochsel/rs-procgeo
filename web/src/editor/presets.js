// ─────────────────────────────────────────────────────────────────────────────
// Preset graphs — the starter scene plus a handful of example networks.
// Each graph is in the same serialized form produced by Graph.toJSON().
// Params not listed fall back to the node's registry defaults.
// ─────────────────────────────────────────────────────────────────────────────

const node = (id, type, x, y, params = {}, name) => ({ id, type, x, y, name, params });
const link = (from, to, port = 0) => ({ from, to, port });

export const STARTER_GRAPH = {
    version: 1,
    nodes: [
        node('n1', 'box', 60, 40),
        node('n2', 'subdivide', 60, 170, { depth: 2, mode: 'catmullClark' }),
        node('n3', 'transform', 60, 300, { rotate: [0, 25, 0] }),
    ],
    connections: [link('n1', 'n2'), link('n2', 'n3')],
    display: 'n3',
};

export const PRESETS = {
    starter: { label: 'Starter — Rounded Cube', graph: STARTER_GRAPH },

    terrain: {
        label: 'Noise Terrain',
        graph: {
            nodes: [
                node('n1', 'grid', 60, 40, { rows: 80, cols: 80, sizeX: 6, sizeY: 6 }),
                node('n2', 'displace', 60, 180, {
                    direction: 'y', coordinates: 'boundingBox', projection: 'xz',
                    strength: 1.2, noiseType: 'simplex', fractal: 'terrain',
                    scale: [1.4, 1.4, 1.4], octaves: 6,
                }),
                node('n3', 'color', 60, 320, { color: [0.35, 0.55, 0.32] }),
            ],
            connections: [link('n1', 'n2'), link('n2', 'n3')],
            display: 'n3',
        },
    },

    instances: {
        label: 'Scatter & Copy',
        graph: {
            nodes: [
                node('n1', 'grid', 60, 40, { rows: 20, cols: 20, sizeX: 5, sizeY: 5 }),
                node('n2', 'scatter', 60, 180, { count: 150, seed: 3 }),
                node('n3', 'box', 300, 40, { size: [0.15, 0.4, 0.15] }),
                node('n4', 'copyToPoints', 180, 320),
            ],
            // copyToPoints: port 0 = instance, port 1 = target points
            connections: [link('n1', 'n2'), link('n3', 'n4', 0), link('n2', 'n4', 1)],
            display: 'n4',
        },
    },

    boolean: {
        label: 'Boolean Subtract',
        graph: {
            nodes: [
                node('n1', 'box', 60, 40, { size: [1, 1, 1] }),
                node('n2', 'sphere', 300, 40, { radius: 0.62, rows: 24, cols: 32 }),
                node('n3', 'booleanOp', 160, 200, { operation: 'subtract' }),
                node('n4', 'normal', 160, 330),
            ],
            connections: [link('n1', 'n3', 0), link('n2', 'n3', 1), link('n3', 'n4')],
            display: 'n4',
        },
    },

    fracture: {
        label: 'Voronoi Fracture',
        graph: {
            nodes: [
                node('n1', 'sphere', 60, 40, { radius: 0.8, rows: 20, cols: 28 }),
                node('n2', 'box', 320, 40, { size: [1.6, 1.6, 1.6] }),
                node('n3', 'scatter', 320, 170, { count: 24, seed: 7 }),
                node('n4', 'voronoiFracture', 160, 320),
            ],
            connections: [link('n1', 'n4', 0), link('n2', 'n3'), link('n3', 'n4', 1)],
            display: 'n4',
        },
    },

    extrude: {
        label: 'Extruded Panels',
        graph: {
            nodes: [
                node('n1', 'grid', 60, 40, { rows: 6, cols: 6, sizeX: 4, sizeY: 4 }),
                node('n2', 'polyExtrude', 60, 180, { distance: 0.5, inset: 0.08 }),
                node('n3', 'subdivide', 60, 320, { depth: 1, mode: 'linear' }),
            ],
            connections: [link('n1', 'n2'), link('n2', 'n3')],
            display: 'n2',
        },
    },
};
