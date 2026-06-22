// ─────────────────────────────────────────────────────────────────────────────
// Node registry — the catalog of SOP node types for the visual editor.
//
// Each node type maps to a function on the procgeo WASM module. A node has:
//   - inputs:  ordered geometry input ports (label + required flag)
//   - params:  parameter schema (name matches the WASM param key where possible)
//   - make:    (pg, inputs, params) => Geometry   evaluation function
//
// Most SOPs accept a params object whose keys match the param names declared
// here, so the default `make` simply forwards them. Nodes with nested params
// (e.g. displace's `noise`) provide a custom `make`.
// ─────────────────────────────────────────────────────────────────────────────

// Param helpers — keep node definitions terse and readable.
const f = (name, label, def, o = {}) => ({ name, label, type: 'float', default: def, ...o });
const int = (name, label, def, o = {}) => ({ name, label, type: 'int', default: def, ...o });
const bool = (name, label, def) => ({ name, label, type: 'bool', default: def });
const v3 = (name, label, def) => ({ name, label, type: 'vec3', default: def });
const v2 = (name, label, def) => ({ name, label, type: 'vec2', default: def });
const en = (name, label, def, options) => ({ name, label, type: 'enum', default: def, options });
const str = (name, label, def) => ({ name, label, type: 'string', default: def });
const col = (name, label, def) => ({ name, label, type: 'color', default: def });

const IN = (label, required = true) => ({ label, required });

// Build a plain params object from a node's schema + current values.
// Vec params are already stored as arrays, so they pass straight through.
function paramsObject(def, values) {
    const out = {};
    for (const p of def.params) {
        out[p.name] = values[p.name];
    }
    return out;
}

// ── Node definitions ─────────────────────────────────────────────────────────
// Order within a category controls menu ordering.
const NODE_DEFS = [
    // ---- Creation (no inputs) ----
    {
        type: 'box', label: 'Box', category: 'Creation', inputs: [],
        params: [v3('size', 'Size', [1, 1, 1]), v3('center', 'Center', [0, 0, 0])],
        make: (pg, _i, p) => pg.createBox(p),
    },
    {
        type: 'grid', label: 'Grid', category: 'Creation', inputs: [],
        params: [
            int('rows', 'Rows', 10, { min: 1 }), int('cols', 'Columns', 10, { min: 1 }),
            f('sizeX', 'Size X', 1), f('sizeY', 'Size Y', 1), v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createGrid(p),
    },
    {
        type: 'sphere', label: 'Sphere', category: 'Creation', inputs: [],
        params: [
            f('radius', 'Radius', 0.5), int('rows', 'Rows', 13, { min: 2 }),
            int('cols', 'Columns', 24, { min: 3 }), v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createSphere(p),
    },
    {
        type: 'icosphere', label: 'Icosphere', category: 'Creation', inputs: [],
        params: [
            f('radius', 'Radius', 0.5), int('subdivisions', 'Subdivisions', 2, { min: 0, max: 6 }),
            v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createIcosphere(p),
    },
    {
        type: 'tube', label: 'Tube', category: 'Creation', inputs: [],
        params: [
            f('radiusBottom', 'Radius Bottom', 0.5), f('radiusTop', 'Radius Top', 0.5),
            f('height', 'Height', 1), int('cols', 'Columns', 12, { min: 3 }),
            int('rows', 'Rows', 1, { min: 1 }), v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createTube(p),
    },
    {
        type: 'torus', label: 'Torus', category: 'Creation', inputs: [],
        params: [
            f('radiusOuter', 'Radius Outer', 0.5), f('radiusInner', 'Radius Inner', 0.2),
            int('rows', 'Rows', 12, { min: 3 }), int('cols', 'Columns', 24, { min: 3 }),
            v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createTorus(p),
    },
    {
        type: 'circle', label: 'Circle', category: 'Creation', inputs: [],
        params: [
            f('radius', 'Radius', 0.5), int('divisions', 'Divisions', 24, { min: 3 }),
            v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createCircle(p),
    },
    {
        type: 'line', label: 'Line', category: 'Creation', inputs: [],
        params: [
            v3('origin', 'Origin', [0, 0, 0]), v3('direction', 'Direction', [0, 1, 0]),
            f('length', 'Length', 1), int('points', 'Points', 2, { min: 2 }),
        ],
        make: (pg, _i, p) => pg.createLine(p),
    },
    {
        type: 'spiral', label: 'Spiral', category: 'Creation', inputs: [],
        params: [
            f('startRadius', 'Start Radius', 0.1), f('endRadius', 'End Radius', 1),
            f('height', 'Height', 1), f('turns', 'Turns', 3),
            int('points', 'Points', 100, { min: 2 }), v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createSpiral(p),
    },
    {
        type: 'helix', label: 'Helix', category: 'Creation', inputs: [],
        params: [
            f('radius', 'Radius', 0.5), f('height', 'Height', 1), f('turns', 'Turns', 3),
            int('points', 'Points', 100, { min: 2 }), v3('center', 'Center', [0, 0, 0]),
        ],
        make: (pg, _i, p) => pg.createHelix(p),
    },
    {
        type: 'teapot', label: 'Teapot', category: 'Creation', inputs: [],
        params: [v3('size', 'Size', [1, 1, 1]), v3('center', 'Center', [0, 0, 0]), int('resolution', 'Resolution', 6, { min: 1, max: 16 })],
        make: (pg, _i, p) => pg.createTeapot(p),
    },

    // ---- Transform / Deform ----
    {
        type: 'transform', label: 'Transform', category: 'Transform', inputs: [IN('Geometry')],
        params: [
            v3('translate', 'Translate', [0, 0, 0]), v3('rotate', 'Rotate', [0, 0, 0]),
            v3('scale', 'Scale', [1, 1, 1]), v3('pivot', 'Pivot', [0, 0, 0]),
        ],
        make: (pg, i, p) => pg.transform(i[0], p),
    },
    {
        type: 'normal', label: 'Normal', category: 'Transform', inputs: [IN('Geometry')],
        params: [
            bool('makeUnitLength', 'Make Unit Length', true), bool('reverseNormals', 'Reverse Normals', false),
            f('cuspAngle', 'Cusp Angle', 60),
        ],
        make: (pg, i, p) => pg.computeNormals(i[0], { computeNormals: true, ...p }),
    },
    {
        type: 'displace', label: 'Displace (Noise)', category: 'Transform', inputs: [IN('Geometry')],
        params: [
            f('strength', 'Strength', 0.3), f('midlevel', 'Mid Level', 0.5),
            en('direction', 'Direction', 'normal', ['normal', 'x', 'y', 'z']),
            en('coordinates', 'Coordinates', 'auto', ['auto', 'uv', 'boundingBox', 'position']),
            en('projection', 'Projection', 'xz', ['xy', 'xz', 'yz']),
            en('noiseType', 'Noise Type', 'simplex', ['perlin', 'simplex', 'worley', 'worleyF2F1']),
            en('fractal', 'Fractal', 'standard', ['none', 'standard', 'terrain']),
            v3('scale', 'Noise Scale', [2, 2, 2]),
            int('octaves', 'Octaves', 4, { min: 1, max: 10 }),
            f('lacunarity', 'Lacunarity', 2), f('roughness', 'Roughness', 0.5),
            int('seed', 'Seed', 0),
        ],
        make: (pg, i, p) => pg.displace(i[0], {
            strength: p.strength, midlevel: p.midlevel, direction: p.direction,
            coordinates: p.coordinates, projection: p.projection,
            noise: {
                noiseType: p.noiseType, fractal: p.fractal, scale: p.scale,
                octaves: p.octaves, lacunarity: p.lacunarity, roughness: p.roughness, seed: p.seed,
            },
        }),
    },
    {
        type: 'bend', label: 'Bend', category: 'Transform', inputs: [IN('Geometry')],
        params: [
            bool('bendEnable', 'Bend Enable', true),
            en('bendMode', 'Bend Mode', 'angle', ['angle', 'direction']),
            f('bendAngle', 'Bend Angle', 45),
            bool('twistEnable', 'Twist Enable', false), f('twistAngle', 'Twist Angle', 0),
            v3('upVector', 'Up Vector', [0, 1, 0]),
            v3('captureOrigin', 'Capture Origin', [0, 0, 0]),
            v3('captureDirection', 'Capture Direction', [0, 1, 0]),
            f('captureLength', 'Capture Length', 1),
        ],
        make: (pg, i, p) => pg.bend(i[0], p),
    },

    // ---- Reshape / Remesh ----
    {
        type: 'subdivide', label: 'Subdivide', category: 'Reshape', inputs: [IN('Geometry')],
        params: [int('depth', 'Depth', 1, { min: 0, max: 5 }), en('mode', 'Mode', 'catmullClark', ['linear', 'catmullClark'])],
        make: (pg, i, p) => pg.subdivide(i[0], p),
    },
    {
        type: 'polyExtrude', label: 'Poly Extrude', category: 'Reshape', inputs: [IN('Geometry')],
        params: [
            f('distance', 'Distance', 0.2), f('inset', 'Inset', 0),
            bool('outputFront', 'Output Front', true), bool('outputSide', 'Output Side', true),
        ],
        make: (pg, i, p) => pg.polyExtrude(i[0], p),
    },
    {
        type: 'smooth', label: 'Smooth', category: 'Reshape', inputs: [IN('Geometry')],
        params: [int('iterations', 'Iterations', 10, { min: 1 }), f('strength', 'Strength', 0.5)],
        make: (pg, i, p) => pg.smooth(i[0], p),
    },
    {
        type: 'clip', label: 'Clip', category: 'Reshape', inputs: [IN('Geometry')],
        params: [
            v3('origin', 'Origin', [0, 0, 0]), v3('normal', 'Normal', [0, 1, 0]),
            bool('keepAbove', 'Keep Above', true), bool('createCap', 'Create Cap', true),
        ],
        make: (pg, i, p) => pg.clip(i[0], p),
    },
    {
        type: 'polyBevel', label: 'Poly Bevel', category: 'Reshape', inputs: [IN('Geometry')],
        params: [f('offset', 'Offset', 0.1), int('divisions', 'Divisions', 1, { min: 1 })],
        make: (pg, i, p) => pg.polyBevel(i[0], p),
    },
    {
        type: 'polyWire', label: 'Poly Wire', category: 'Reshape', inputs: [IN('Geometry')],
        params: [f('radius', 'Radius', 0.05), int('divisions', 'Divisions', 6, { min: 3 })],
        make: (pg, i, p) => pg.polyWire(i[0], p),
    },
    {
        type: 'polyReduce', label: 'Poly Reduce', category: 'Reshape', inputs: [IN('Geometry')],
        params: [f('targetPercent', 'Target %', 50, { min: 1, max: 100 }), bool('preserveBoundaries', 'Preserve Boundaries', true)],
        make: (pg, i, p) => pg.polyReduce(i[0], p),
    },
    {
        type: 'resample', label: 'Resample', category: 'Reshape', inputs: [IN('Geometry')],
        params: [f('length', 'Length', 0.1), int('maxSegments', 'Max Segments', 1000, { min: 1 })],
        make: (pg, i, p) => pg.resample(i[0], p),
    },
    {
        type: 'revolve', label: 'Revolve', category: 'Reshape', inputs: [IN('Curve')],
        params: [
            v3('origin', 'Origin', [0, 0, 0]), v3('axis', 'Axis', [0, 1, 0]),
            int('divisions', 'Divisions', 12, { min: 3 }),
            f('startAngle', 'Start Angle', 0), f('endAngle', 'End Angle', 360),
            bool('endCaps', 'End Caps', false),
        ],
        make: (pg, i, p) => pg.revolve(i[0], p),
    },

    // ---- Copy / Scatter / Merge ----
    {
        type: 'scatter', label: 'Scatter', category: 'Copy', inputs: [IN('Surface')],
        params: [int('count', 'Count', 100, { min: 1 }), int('seed', 'Seed', 0)],
        make: (pg, i, p) => pg.scatter(i[0], p),
    },
    {
        type: 'copyToPoints', label: 'Copy To Points', category: 'Copy', inputs: [IN('Instance'), IN('Target Points')],
        params: [],
        make: (pg, i) => pg.copyToPoints(i[0], i[1]),
    },
    {
        type: 'merge', label: 'Merge', category: 'Copy', inputs: [IN('Input A'), IN('Input B')],
        params: [],
        make: (pg, i) => pg.merge(i[0], i[1]),
    },

    // ---- Topology ----
    {
        type: 'reverse', label: 'Reverse', category: 'Topology', inputs: [IN('Geometry')],
        params: [],
        make: (pg, i) => pg.reverse(i[0]),
    },
    {
        type: 'fuse', label: 'Fuse', category: 'Topology', inputs: [IN('Geometry')],
        params: [f('distance', 'Distance', 0.001)],
        make: (pg, i, p) => pg.fuse(i[0], p),
    },
    {
        type: 'sort', label: 'Sort', category: 'Topology', inputs: [IN('Geometry')],
        params: [int('seed', 'Seed', 0)],
        make: (pg, i, p) => pg.sort(i[0], p),
    },
    {
        type: 'connectivity', label: 'Connectivity', category: 'Topology', inputs: [IN('Geometry')],
        params: [str('attribName', 'Attribute Name', 'class')],
        make: (pg, i, p) => pg.connectivity(i[0], p),
    },

    // ---- Boolean / Fracture ----
    {
        type: 'booleanOp', label: 'Boolean', category: 'Boolean', inputs: [IN('Input A'), IN('Input B')],
        params: [
            en('operation', 'Operation', 'union', ['union', 'intersect', 'subtract']),
            en('treatAAs', 'Treat A As', 'solid', ['solid', 'surface']),
            en('treatBAs', 'Treat B As', 'solid', ['solid', 'surface']),
        ],
        make: (pg, i, p) => pg.booleanOp(i[0], i[1], p),
    },
    {
        type: 'voronoiFracture', label: 'Voronoi Fracture', category: 'Boolean', inputs: [IN('Geometry'), IN('Cell Points')],
        params: [f('cutPlaneOffset', 'Cut Plane Offset', 0), bool('createInsideFaces', 'Create Inside Faces', true)],
        make: (pg, i, p) => pg.voronoiFracture(i[0], i[1], p),
    },

    // ---- Attributes / Color ----
    {
        type: 'color', label: 'Color', category: 'Attributes', inputs: [IN('Geometry')],
        params: [col('color', 'Color', [0.27, 0.53, 0.8])],
        make: (pg, i, p) => pg.color(i[0], p),
    },
    {
        type: 'attribNoise', label: 'Attrib Noise', category: 'Attributes', inputs: [IN('Geometry')],
        params: [
            str('attribName', 'Attribute', 'P'),
            en('noiseType', 'Noise Type', 'perlin', ['perlin', 'simplex', 'worley', 'worleyF2F1']),
            en('operation', 'Operation', 'add', ['setInitial', 'set', 'add', 'subtract', 'multiply', 'min', 'max']),
            f('amplitude', 'Amplitude', 0.2), f('elementSize', 'Element Size', 1),
            int('seed', 'Seed', 0), int('dimensions', 'Dimensions', 3, { min: 1, max: 3 }),
        ],
        make: (pg, i, p) => pg.attribNoise(i[0], p),
    },
    {
        type: 'attribRandomize', label: 'Attrib Randomize', category: 'Attributes', inputs: [IN('Geometry')],
        params: [
            str('attribName', 'Attribute', 'Cd'),
            en('class', 'Class', 'Point', ['Point', 'Vertex', 'Primitive', 'Detail']),
            en('distribution', 'Distribution', 'Uniform', ['Uniform', 'Gaussian', 'Bernoulli']),
            en('operation', 'Operation', 'Set', ['Set', 'Add', 'Multiply']),
            int('seed', 'Seed', 0), f('minValue', 'Min', 0), f('maxValue', 'Max', 1),
            int('dimensions', 'Dimensions', 3, { min: 1, max: 3 }),
        ],
        make: (pg, i, p) => pg.attribRandomize(i[0], p),
    },

    // ---- Groups ----
    {
        type: 'groupCreate', label: 'Group Create', category: 'Groups', inputs: [IN('Geometry')],
        params: [
            str('name', 'Name', 'group1'),
            en('groupType', 'Group Type', 'points', ['points', 'primitives']),
            en('mode', 'Mode', 'boundingBox', ['range', 'boundingBox', 'normal']),
            int('rangeStart', 'Range Start', 0), int('rangeEnd', 'Range End', 0),
            v3('bboxMin', 'BBox Min', [-1, -1, -1]), v3('bboxMax', 'BBox Max', [1, 1, 1]),
            v3('normalDirection', 'Normal Dir', [0, 1, 0]), f('normalAngle', 'Normal Angle', 45),
        ],
        make: (pg, i, p) => pg.groupCreate(i[0], p),
    },
    {
        type: 'blast', label: 'Blast', category: 'Groups', inputs: [IN('Geometry')],
        params: [
            str('groupName', 'Group Name', 'group1'),
            en('entity', 'Entity', 'points', ['points', 'primitives']),
            bool('negate', 'Negate (delete non-selected)', false),
        ],
        make: (pg, i, p) => pg.blast(i[0], p),
    },
];

// Index by type for fast lookup.
export const NODE_REGISTRY = new Map(NODE_DEFS.map((d) => [d.type, d]));

// Ordered list of categories for the add-node menu.
export const NODE_CATEGORIES = (() => {
    const cats = new Map();
    for (const d of NODE_DEFS) {
        if (!cats.has(d.category)) cats.set(d.category, []);
        cats.get(d.category).push(d);
    }
    return cats;
})();

export function getNodeDef(type) {
    return NODE_REGISTRY.get(type);
}

// Default parameter values for a node type (deep-copies arrays).
export function defaultParams(type) {
    const def = getNodeDef(type);
    const values = {};
    for (const p of def.params) {
        values[p.name] = Array.isArray(p.default) ? p.default.slice() : p.default;
    }
    return values;
}

export { paramsObject };
