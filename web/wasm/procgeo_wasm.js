/* @ts-self-types="./procgeo_wasm.d.ts" */

/**
 * Geometry wrapper exposed to JS via WASM.
 */
export class Geometry {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(Geometry.prototype);
        obj.__wbg_ptr = ptr;
        GeometryFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GeometryFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_geometry_free(ptr, 0);
    }
    /**
     * @returns {any}
     */
    boundingBox() {
        const ret = wasm.geometry_boundingBox(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get colors as a flat Float32Array (if "Cd" attribute exists).
     * @returns {Float32Array | undefined}
     */
    getColors() {
        const ret = wasm.geometry_getColors(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        }
        return v1;
    }
    /**
     * Get normals as a flat Float32Array (if "N" attribute exists).
     * @returns {Float32Array | undefined}
     */
    getNormals() {
        const ret = wasm.geometry_getNormals(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        }
        return v1;
    }
    /**
     * Get all point positions as a flat Float32Array [x0,y0,z0, x1,y1,z1, ...]
     * Useful for feeding directly to WebGL/Three.js BufferGeometry.
     * @returns {Float32Array}
     */
    getPositions() {
        const ret = wasm.geometry_getPositions(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get triangle indices as a flat Uint32Array (fan-triangulated).
     * Useful for WebGL/Three.js index buffers.
     * @returns {Uint32Array}
     */
    getTriangleIndices() {
        const ret = wasm.geometry_getTriangleIndices(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    constructor() {
        const ret = wasm.geometry_new();
        this.__wbg_ptr = ret >>> 0;
        GeometryFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {number}
     */
    get numPoints() {
        const ret = wasm.geometry_numPoints(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get numPrims() {
        const ret = wasm.geometry_numPrims(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get numVertices() {
        const ret = wasm.geometry_numVertices(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} index
     * @returns {Float32Array}
     */
    pointPos(index) {
        const ret = wasm.geometry_pointPos(this.__wbg_ptr, index);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Write geometry as GLB bytes (Uint8Array).
     * @returns {Uint8Array}
     */
    toGlb() {
        const ret = wasm.geometry_toGlb(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Write geometry as OBJ string.
     * @returns {string}
     */
    toObj() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.geometry_toObj(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
}
if (Symbol.dispose) Geometry.prototype[Symbol.dispose] = Geometry.prototype.free;

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function attribBlur(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribBlur(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} dest
 * @param {Geometry | null} [source]
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function attribCopy(dest, source, params) {
    _assertClass(dest, Geometry);
    let ptr0 = 0;
    if (!isLikeNone(source)) {
        _assertClass(source, Geometry);
        ptr0 = source.__destroy_into_raw();
    }
    const ret = wasm.attribCopy(dest.__wbg_ptr, ptr0, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function attribFill(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribFill(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function attribNoise(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribNoise(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function attribRandomize(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribRandomize(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function attribSort(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribSort(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} dest
 * @param {Geometry} source
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function attribTransfer(dest, source, params) {
    _assertClass(dest, Geometry);
    _assertClass(source, Geometry);
    const ret = wasm.attribTransfer(dest.__wbg_ptr, source.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function clip(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.clip(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function color(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.color(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @returns {Geometry}
 */
export function computeNormals(geo) {
    _assertClass(geo, Geometry);
    const ret = wasm.computeNormals(geo.__wbg_ptr);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} source
 * @param {Geometry} target
 * @returns {Geometry}
 */
export function copyToPoints(source, target) {
    _assertClass(source, Geometry);
    _assertClass(target, Geometry);
    const ret = wasm.copyToPoints(source.__wbg_ptr, target.__wbg_ptr);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function createBox(params) {
    const ret = wasm.createBox(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function createCircle(params) {
    const ret = wasm.createCircle(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function createGrid(params) {
    const ret = wasm.createGrid(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function createLine(params) {
    const ret = wasm.createLine(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function createSphere(params) {
    const ret = wasm.createSphere(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function createTorus(params) {
    const ret = wasm.createTorus(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function createTube(params) {
    const ret = wasm.createTube(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function fuse(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.fuse(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function polyExtrude(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.polyExtrude(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @returns {Geometry}
 */
export function reverse(geo) {
    _assertClass(geo, Geometry);
    const ret = wasm.reverse(geo.__wbg_ptr);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function scatter(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.scatter(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function smooth(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.smooth(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function subdivide(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.subdivide(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function transform(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.transform(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {Geometry} geo
 * @param {Geometry} points
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function voronoiFracture(geo, points, params) {
    _assertClass(geo, Geometry);
    _assertClass(points, Geometry);
    const ret = wasm.voronoiFracture(geo.__wbg_ptr, points.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_2e59b1b37a9a34c3: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg___wbindgen_boolean_get_a86c216575a75c30: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_dd5d2d07ce9e6c57: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_undefined_c0cca72b82b86f4d: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_number_get_7579aab02a8a620c: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_914df97fcfa788f2: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_81fc77679af83bc6: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_get_4848e350b40afc16: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_f96702c6245e4ef9: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_isArray_db61795ad004c139: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_new_4f9fafbb3909af72: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_from_slice_d85ad974cf8f6f35: function(arg0, arg1) {
            const ret = new Float32Array(getArrayF32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_set_8ee2d34facb8466e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./procgeo_wasm_bg.js": import0,
    };
}

const GeometryFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_geometry_free(ptr >>> 0, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('procgeo_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
