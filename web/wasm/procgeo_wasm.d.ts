/* tslint:disable */
/* eslint-disable */

/**
 * Geometry wrapper exposed to JS via WASM.
 */
export class Geometry {
    free(): void;
    [Symbol.dispose](): void;
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
     * Write geometry as GLB bytes (Uint8Array).
     */
    toGlb(): Uint8Array;
    /**
     * Write geometry as OBJ string.
     */
    toObj(): string;
    readonly numPoints: number;
    readonly numPrims: number;
    readonly numVertices: number;
}

export function attribBlur(geo: Geometry, params?: any | null): Geometry;

export function attribCopy(dest: Geometry, source?: Geometry | null, params?: any | null): Geometry;

export function attribFill(geo: Geometry, params?: any | null): Geometry;

export function attribNoise(geo: Geometry, params?: any | null): Geometry;

export function attribRandomize(geo: Geometry, params?: any | null): Geometry;

export function attribSort(geo: Geometry, params?: any | null): Geometry;

export function attribTransfer(dest: Geometry, source: Geometry, params?: any | null): Geometry;

export function clip(geo: Geometry, params?: any | null): Geometry;

export function color(geo: Geometry, params?: any | null): Geometry;

export function computeNormals(geo: Geometry): Geometry;

export function copyToPoints(source: Geometry, target: Geometry): Geometry;

export function createBox(params?: any | null): Geometry;

export function createCircle(params?: any | null): Geometry;

export function createGrid(params?: any | null): Geometry;

export function createLine(params?: any | null): Geometry;

export function createSphere(params?: any | null): Geometry;

export function createTorus(params?: any | null): Geometry;

export function createTube(params?: any | null): Geometry;

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

/**
 * List all registered SOP names.
 */
export function listSops(): string[];

export function polyExtrude(geo: Geometry, params?: any | null): Geometry;

export function reverse(geo: Geometry): Geometry;

export function scatter(geo: Geometry, params?: any | null): Geometry;

export function smooth(geo: Geometry, params?: any | null): Geometry;

export function subdivide(geo: Geometry, params?: any | null): Geometry;

export function transform(geo: Geometry, params?: any | null): Geometry;

export function voronoiFracture(geo: Geometry, points: Geometry, params?: any | null): Geometry;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_geometry_free: (a: number, b: number) => void;
    readonly attribBlur: (a: number, b: number) => [number, number, number];
    readonly attribCopy: (a: number, b: number, c: number) => [number, number, number];
    readonly attribFill: (a: number, b: number) => [number, number, number];
    readonly attribNoise: (a: number, b: number) => [number, number, number];
    readonly attribRandomize: (a: number, b: number) => [number, number, number];
    readonly attribSort: (a: number, b: number) => [number, number, number];
    readonly attribTransfer: (a: number, b: number, c: number) => [number, number, number];
    readonly clip: (a: number, b: number) => [number, number, number];
    readonly color: (a: number, b: number) => [number, number, number];
    readonly computeNormals: (a: number) => [number, number, number];
    readonly copyToPoints: (a: number, b: number) => [number, number, number];
    readonly createBox: (a: number) => [number, number, number];
    readonly createCircle: (a: number) => [number, number, number];
    readonly createGrid: (a: number) => [number, number, number];
    readonly createLine: (a: number) => [number, number, number];
    readonly createSphere: (a: number) => [number, number, number];
    readonly createTorus: (a: number) => [number, number, number];
    readonly createTube: (a: number) => [number, number, number];
    readonly executeSop: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly executeSopCreate: (a: number, b: number, c: number) => [number, number, number];
    readonly fuse: (a: number, b: number) => [number, number, number];
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
    readonly geometry_toGlb: (a: number) => [number, number, number, number];
    readonly geometry_toObj: (a: number) => [number, number, number, number];
    readonly listSops: () => [number, number];
    readonly polyExtrude: (a: number, b: number) => [number, number, number];
    readonly reverse: (a: number) => [number, number, number];
    readonly scatter: (a: number, b: number) => [number, number, number];
    readonly smooth: (a: number, b: number) => [number, number, number];
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
