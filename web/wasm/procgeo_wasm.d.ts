/* tslint:disable */
/* eslint-disable */

/**
 * Geometry wrapper exposed to JS via WASM.
 */
export class Geometry {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Create a closed face (polygon) from an array of point indices. Returns the primitive index.
     */
    addFace(point_indices: Uint32Array): number;
    /**
     * Add a point at position [x, y, z]. Returns the point index.
     */
    addPoint(x: number, y: number, z: number): number;
    /**
     * Create an open polyline from an array of point indices. Returns the primitive index.
     */
    addPolyline(point_indices: Uint32Array): number;
    /**
     * Get all values of a numeric attribute as a flat Float64Array.
     * Components interleaved: for vec3 → [x0,y0,z0, x1,y1,z1, ...].
     */
    attribData(_class: string, name: string): Float64Array | undefined;
    /**
     * Get all values of a string attribute.
     */
    attribDataString(_class: string, name: string): string[] | undefined;
    /**
     * List attribute names for a class ("point", "vertex", "primitive", "detail").
     */
    attribNames(_class: string): string[];
    /**
     * Get the component count of an attribute (1 for float, 3 for vec3, etc.).
     */
    attribSize(_class: string, name: string): number | undefined;
    /**
     * Get the type name of an attribute ("Float", "Int", "Vector3", etc.).
     */
    attribType(_class: string, name: string): string | undefined;
    boundingBox(): any;
    /**
     * Get colors as a flat Float32Array (if "Cd" attribute exists).
     */
    getColors(): Float32Array | undefined;
    /**
     * Get normals as a flat Float32Array (if "N" attribute exists).
     */
    getNormals(): Float32Array | undefined;
    /**
     * Get all point positions as a flat Float32Array [x0,y0,z0, x1,y1,z1, ...]
     * Useful for feeding directly to WebGL/Three.js BufferGeometry.
     */
    getPositions(): Float32Array;
    /**
     * Get triangle indices as a flat Uint32Array (fan-triangulated).
     * Useful for WebGL/Three.js index buffers.
     */
    getTriangleIndices(): Uint32Array;
    constructor();
    pointPos(index: number): Float32Array;
    /**
     * Get the point indices for a specific primitive.
     */
    primPointIndices(prim_index: number): Uint32Array;
    /**
     * Get the number of vertices in a specific primitive.
     */
    primVertexCount(prim_index: number): number;
    /**
     * Set the position of an existing point.
     */
    setPointPos(index: number, x: number, y: number, z: number): void;
    /**
     * Write geometry as GLB bytes (Uint8Array).
     */
    toGlb(): Uint8Array;
    /**
     * Write geometry as OBJ string.
     */
    toObj(): string;
    /**
     * Get which point a vertex maps to.
     */
    vertexPoint(vertex_index: number): number;
    readonly numPoints: number;
    readonly numPrims: number;
    readonly numVertices: number;
}

export function attribBlur(geo: Geometry, params?: any | null): Geometry;

export function attribCopy(dest: Geometry, source?: Geometry | null, params?: any | null): Geometry;

export function attribCreate(geo: Geometry, params?: any | null): Geometry;

export function attribDelete(geo: Geometry, params?: any | null): Geometry;

export function attribFill(geo: Geometry, params?: any | null): Geometry;

export function attribNoise(geo: Geometry, params?: any | null): Geometry;

export function attribPromote(geo: Geometry, params?: any | null): Geometry;

export function attribRandomize(geo: Geometry, params?: any | null): Geometry;

export function attribRename(geo: Geometry, params?: any | null): Geometry;

export function attribSort(geo: Geometry, params?: any | null): Geometry;

export function attribTransfer(dest: Geometry, source: Geometry, params?: any | null): Geometry;

export function blast(geo: Geometry, params?: any | null): Geometry;

export function clip(geo: Geometry, params?: any | null): Geometry;

export function color(geo: Geometry, params?: any | null): Geometry;

export function computeNormals(geo: Geometry): Geometry;

export function connectivity(geo: Geometry, params?: any | null): Geometry;

export function copyToPoints(source: Geometry, target: Geometry): Geometry;

export function createBox(params?: any | null): Geometry;

export function createCircle(params?: any | null): Geometry;

export function createGrid(params?: any | null): Geometry;

export function createLine(params?: any | null): Geometry;

export function createMetaball(params?: any | null): Geometry;

export function createSphere(params?: any | null): Geometry;

export function createTorus(params?: any | null): Geometry;

export function createTube(params?: any | null): Geometry;

export function deleteSop(geo: Geometry, params?: any | null): Geometry;

export function enumerateAttrib(geo: Geometry, params?: any | null): Geometry;

/**
 * Execute any registered SOP by name. Params are a JSON-compatible JS object.
 * Uses Rust/snake_case field names for params (matching serde serialization).
 */
export function executeSop(name: string, geo: Geometry, params?: any | null): Geometry;

/**
 * Execute a creation SOP (no input geometry required).
 */
export function executeSopCreate(name: string, params?: any | null): Geometry;

export function fuse(geo: Geometry, params?: any | null): Geometry;

export function groupCombine(geo: Geometry, params?: any | null): Geometry;

export function groupCreate(geo: Geometry, params?: any | null): Geometry;

/**
 * List all registered SOP names.
 */
export function listSops(): string[];

export function measure(geo: Geometry, params?: any | null): Geometry;

/**
 * Merge two geometries into one. Chain calls to merge more:
 * `merge(merge(a, b), c)`.
 */
export function merge(a: Geometry, b: Geometry): Geometry;

export function polyBevel(geo: Geometry, params?: any | null): Geometry;

export function polyExtrude(geo: Geometry, params?: any | null): Geometry;

export function polyFill(geo: Geometry, params?: any | null): Geometry;

export function polyReduce(geo: Geometry, params?: any | null): Geometry;

export function polyWire(geo: Geometry, params?: any | null): Geometry;

export function resample(geo: Geometry, params?: any | null): Geometry;

export function reverse(geo: Geometry): Geometry;

export function revolve(geo: Geometry, params?: any | null): Geometry;

export function scatter(geo: Geometry, params?: any | null): Geometry;

export function smooth(geo: Geometry, params?: any | null): Geometry;

export function sort(geo: Geometry, params?: any | null): Geometry;

export function subdivide(geo: Geometry, params?: any | null): Geometry;

export function transform(geo: Geometry, params?: any | null): Geometry;

export function voronoiFracture(geo: Geometry, points: Geometry, params?: any | null): Geometry;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_geometry_free: (a: number, b: number) => void;
    readonly attribBlur: (a: number, b: number) => [number, number, number];
    readonly attribCopy: (a: number, b: number, c: number) => [number, number, number];
    readonly attribCreate: (a: number, b: number) => [number, number, number];
    readonly attribDelete: (a: number, b: number) => [number, number, number];
    readonly attribFill: (a: number, b: number) => [number, number, number];
    readonly attribNoise: (a: number, b: number) => [number, number, number];
    readonly attribPromote: (a: number, b: number) => [number, number, number];
    readonly attribRandomize: (a: number, b: number) => [number, number, number];
    readonly attribRename: (a: number, b: number) => [number, number, number];
    readonly attribSort: (a: number, b: number) => [number, number, number];
    readonly attribTransfer: (a: number, b: number, c: number) => [number, number, number];
    readonly blast: (a: number, b: number) => [number, number, number];
    readonly clip: (a: number, b: number) => [number, number, number];
    readonly color: (a: number, b: number) => [number, number, number];
    readonly computeNormals: (a: number) => [number, number, number];
    readonly connectivity: (a: number, b: number) => [number, number, number];
    readonly copyToPoints: (a: number, b: number) => [number, number, number];
    readonly createBox: (a: number) => [number, number, number];
    readonly createCircle: (a: number) => [number, number, number];
    readonly createGrid: (a: number) => [number, number, number];
    readonly createLine: (a: number) => [number, number, number];
    readonly createMetaball: (a: number) => [number, number, number];
    readonly createSphere: (a: number) => [number, number, number];
    readonly createTorus: (a: number) => [number, number, number];
    readonly createTube: (a: number) => [number, number, number];
    readonly deleteSop: (a: number, b: number) => [number, number, number];
    readonly enumerateAttrib: (a: number, b: number) => [number, number, number];
    readonly executeSop: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly executeSopCreate: (a: number, b: number, c: number) => [number, number, number];
    readonly fuse: (a: number, b: number) => [number, number, number];
    readonly geometry_addFace: (a: number, b: number, c: number) => number;
    readonly geometry_addPoint: (a: number, b: number, c: number, d: number) => number;
    readonly geometry_addPolyline: (a: number, b: number, c: number) => number;
    readonly geometry_attribData: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly geometry_attribDataString: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly geometry_attribNames: (a: number, b: number, c: number) => [number, number];
    readonly geometry_attribSize: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly geometry_attribType: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly geometry_boundingBox: (a: number) => any;
    readonly geometry_getColors: (a: number) => [number, number];
    readonly geometry_getNormals: (a: number) => [number, number];
    readonly geometry_getPositions: (a: number) => [number, number];
    readonly geometry_getTriangleIndices: (a: number) => [number, number];
    readonly geometry_new: () => number;
    readonly geometry_numPoints: (a: number) => number;
    readonly geometry_numPrims: (a: number) => number;
    readonly geometry_numVertices: (a: number) => number;
    readonly geometry_pointPos: (a: number, b: number) => [number, number];
    readonly geometry_primPointIndices: (a: number, b: number) => [number, number];
    readonly geometry_primVertexCount: (a: number, b: number) => number;
    readonly geometry_setPointPos: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly geometry_toGlb: (a: number) => [number, number, number, number];
    readonly geometry_toObj: (a: number) => [number, number, number, number];
    readonly geometry_vertexPoint: (a: number, b: number) => number;
    readonly groupCombine: (a: number, b: number) => [number, number, number];
    readonly groupCreate: (a: number, b: number) => [number, number, number];
    readonly listSops: () => [number, number];
    readonly measure: (a: number, b: number) => [number, number, number];
    readonly merge: (a: number, b: number) => [number, number, number];
    readonly polyBevel: (a: number, b: number) => [number, number, number];
    readonly polyExtrude: (a: number, b: number) => [number, number, number];
    readonly polyFill: (a: number, b: number) => [number, number, number];
    readonly polyReduce: (a: number, b: number) => [number, number, number];
    readonly polyWire: (a: number, b: number) => [number, number, number];
    readonly resample: (a: number, b: number) => [number, number, number];
    readonly reverse: (a: number) => [number, number, number];
    readonly revolve: (a: number, b: number) => [number, number, number];
    readonly scatter: (a: number, b: number) => [number, number, number];
    readonly smooth: (a: number, b: number) => [number, number, number];
    readonly sort: (a: number, b: number) => [number, number, number];
    readonly subdivide: (a: number, b: number) => [number, number, number];
    readonly transform: (a: number, b: number) => [number, number, number];
    readonly voronoiFracture: (a: number, b: number, c: number) => [number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
