/* @ts-self-types="./procgeo_wasm.d.ts" */

export class CopImage {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(CopImage.prototype);
        obj.__wbg_ptr = ptr;
        CopImageFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        CopImageFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_copimage_free(ptr, 0);
    }
    /**
     * @returns {Float32Array}
     */
    getPixels() {
        const ret = wasm.copimage_getPixels(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    get height() {
        const ret = wasm.copimage_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get width() {
        const ret = wasm.copimage_width(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) CopImage.prototype[Symbol.dispose] = CopImage.prototype.free;

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
     * Create a closed face (polygon) from an array of point indices. Returns the primitive index.
     * @param {Uint32Array} point_indices
     * @returns {number}
     */
    addFace(point_indices) {
        const ptr0 = passArray32ToWasm0(point_indices, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_addFace(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Add a point at position [x, y, z]. Returns the point index.
     * @param {number} x
     * @param {number} y
     * @param {number} z
     * @returns {number}
     */
    addPoint(x, y, z) {
        const ret = wasm.geometry_addPoint(this.__wbg_ptr, x, y, z);
        return ret >>> 0;
    }
    /**
     * Create an open polyline from an array of point indices. Returns the primitive index.
     * @param {Uint32Array} point_indices
     * @returns {number}
     */
    addPolyline(point_indices) {
        const ptr0 = passArray32ToWasm0(point_indices, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_addPolyline(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Get all values of a numeric attribute as a flat Float64Array.
     * Components interleaved: for vec3 → [x0,y0,z0, x1,y1,z1, ...].
     * @param {string} _class
     * @param {string} name
     * @returns {Float64Array | undefined}
     */
    attribData(_class, name) {
        const ptr0 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_attribData(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        let v3;
        if (ret[0] !== 0) {
            v3 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        }
        return v3;
    }
    /**
     * Get all values of a string attribute.
     * @param {string} _class
     * @param {string} name
     * @returns {string[] | undefined}
     */
    attribDataString(_class, name) {
        const ptr0 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_attribDataString(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        let v3;
        if (ret[0] !== 0) {
            v3 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        }
        return v3;
    }
    /**
     * List attribute names for a class ("point", "vertex", "primitive", "detail").
     * @param {string} _class
     * @returns {string[]}
     */
    attribNames(_class) {
        const ptr0 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_attribNames(this.__wbg_ptr, ptr0, len0);
        var v2 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Get the component count of an attribute (1 for float, 3 for vec3, etc.).
     * @param {string} _class
     * @param {string} name
     * @returns {number | undefined}
     */
    attribSize(_class, name) {
        const ptr0 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_attribSize(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret === 0x100000001 ? undefined : ret;
    }
    /**
     * Get the type name of an attribute ("Float", "Int", "Vector3", etc.).
     * @param {string} _class
     * @param {string} name
     * @returns {string | undefined}
     */
    attribType(_class, name) {
        const ptr0 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_attribType(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        let v3;
        if (ret[0] !== 0) {
            v3 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v3;
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
     * Get the point indices for a specific primitive.
     * @param {number} prim_index
     * @returns {Uint32Array}
     */
    primPointIndices(prim_index) {
        const ret = wasm.geometry_primPointIndices(this.__wbg_ptr, prim_index);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get the number of vertices in a specific primitive.
     * @param {number} prim_index
     * @returns {number}
     */
    primVertexCount(prim_index) {
        const ret = wasm.geometry_primVertexCount(this.__wbg_ptr, prim_index);
        return ret >>> 0;
    }
    /**
     * Set the position of an existing point.
     * @param {number} index
     * @param {number} x
     * @param {number} y
     * @param {number} z
     */
    setPointPos(index, x, y, z) {
        wasm.geometry_setPointPos(this.__wbg_ptr, index, x, y, z);
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
    /**
     * Get which point a vertex maps to.
     * @param {number} vertex_index
     * @returns {number}
     */
    vertexPoint(vertex_index) {
        const ret = wasm.geometry_vertexPoint(this.__wbg_ptr, vertex_index);
        return ret >>> 0;
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
export function attribCreate(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribCreate(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function attribDelete(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribDelete(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function attribPromote(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribPromote(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function attribRename(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.attribRename(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function blast(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.blast(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function connectivity(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.connectivity(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copBlur(image, params) {
    _assertClass(image, CopImage);
    const ret = wasm.copBlur(image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copChannelSwap(image, params) {
    _assertClass(image, CopImage);
    const ret = wasm.copChannelSwap(image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copCheckerboard(params) {
    const ret = wasm.copCheckerboard(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage} a
 * @param {CopImage} b
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copComposite(a, b, params) {
    _assertClass(a, CopImage);
    _assertClass(b, CopImage);
    const ret = wasm.copComposite(a.__wbg_ptr, b.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copConstant(params) {
    const ret = wasm.copConstant(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage | null} [input_a]
 * @param {CopImage | null} [input_b]
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copCustomShader(input_a, input_b, params) {
    let ptr0 = 0;
    if (!isLikeNone(input_a)) {
        _assertClass(input_a, CopImage);
        ptr0 = input_a.__destroy_into_raw();
    }
    let ptr1 = 0;
    if (!isLikeNone(input_b)) {
        _assertClass(input_b, CopImage);
        ptr1 = input_b.__destroy_into_raw();
    }
    const ret = wasm.copCustomShader(ptr0, ptr1, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copFlip(image, params) {
    _assertClass(image, CopImage);
    const ret = wasm.copFlip(image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copLoadImage(params) {
    const ret = wasm.copLoadImage(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copMirror(image, params) {
    _assertClass(image, CopImage);
    const ret = wasm.copMirror(image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copNoise(params) {
    const ret = wasm.copNoise(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copRamp(params) {
    const ret = wasm.copRamp(isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copResize(image, params) {
    _assertClass(image, CopImage);
    const ret = wasm.copResize(image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copRotate(image, params) {
    _assertClass(image, CopImage);
    const ret = wasm.copRotate(image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function copSwirl(image, params) {
    _assertClass(image, CopImage);
    const ret = wasm.copSwirl(image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
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
export function createMetaball(params) {
    const ret = wasm.createMetaball(isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function deleteSop(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.deleteSop(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function enumerateAttrib(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.enumerateAttrib(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * @param {string} name
 * @param {CopImage} image
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function executeCop(name, image, params) {
    const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    _assertClass(image, CopImage);
    const ret = wasm.executeCop(ptr0, len0, image.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {string} name
 * @param {CopImage} image_a
 * @param {CopImage} image_b
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function executeCopComposite(name, image_a, image_b, params) {
    const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    _assertClass(image_a, CopImage);
    _assertClass(image_b, CopImage);
    const ret = wasm.executeCopComposite(ptr0, len0, image_a.__wbg_ptr, image_b.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * @param {string} name
 * @param {any | null} [params]
 * @returns {CopImage}
 */
export function executeCopCreate(name, params) {
    const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.executeCopCreate(ptr0, len0, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CopImage.__wrap(ret[0]);
}

/**
 * Execute any registered SOP by name. Params are a JSON-compatible JS object.
 * Uses Rust/snake_case field names for params (matching serde serialization).
 * @param {string} name
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function executeSop(name, geo, params) {
    const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    _assertClass(geo, Geometry);
    const ret = wasm.executeSop(ptr0, len0, geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * Execute a creation SOP (no input geometry required).
 * @param {string} name
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function executeSopCreate(name, params) {
    const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.executeSopCreate(ptr0, len0, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function groupCombine(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.groupCombine(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function groupCreate(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.groupCreate(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * Initialize the GPU context for COP image processing.
 * Must be called (and awaited) before using any cop* functions.
 * @returns {Promise<void>}
 */
export function initCopGpu() {
    const ret = wasm.initCopGpu();
    return ret;
}

/**
 * @returns {string[]}
 */
export function listCops() {
    const ret = wasm.listCops();
    var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}

/**
 * List all registered SOP names.
 * @returns {string[]}
 */
export function listSops() {
    const ret = wasm.listSops();
    var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}

/**
 * @param {Geometry} geo
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function measure(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.measure(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Geometry.__wrap(ret[0]);
}

/**
 * Merge two geometries into one. Chain calls to merge more:
 * `merge(merge(a, b), c)`.
 * @param {Geometry} a
 * @param {Geometry} b
 * @returns {Geometry}
 */
export function merge(a, b) {
    _assertClass(a, Geometry);
    _assertClass(b, Geometry);
    const ret = wasm.merge(a.__wbg_ptr, b.__wbg_ptr);
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
export function polyBevel(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.polyBevel(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
 * @param {any | null} [params]
 * @returns {Geometry}
 */
export function polyFill(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.polyFill(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function polyReduce(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.polyReduce(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function polyWire(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.polyWire(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function resample(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.resample(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function revolve(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.revolve(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
export function sort(geo, params) {
    _assertClass(geo, Geometry);
    const ret = wasm.sort(geo.__wbg_ptr, isLikeNone(params) ? 0 : addToExternrefTable0(params));
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
        __wbg_Window_412fe051c1aa1519: function(arg0) {
            const ret = arg0.Window;
            return ret;
        },
        __wbg_WorkerGlobalScope_349300f9b277afe1: function(arg0) {
            const ret = arg0.WorkerGlobalScope;
            return ret;
        },
        __wbg___wbindgen_bigint_get_as_i64_2c5082002e4826e2: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
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
        __wbg___wbindgen_in_4bd7a57e54337366: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_6c98f7e945dacdde: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_function_49868bde5eb1e745: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_344c8750a8525473: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_40c5a80572e8f9d3: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_c0cca72b82b86f4d: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_7d430e744a913d26: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_3a72ae764d46d944: function(arg0, arg1) {
            const ret = arg0 == arg1;
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
        __wbg__wbg_cb_unref_3c3b4f651835fbcb: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_beginComputePass_097033d61ef8af0f: function(arg0, arg1) {
            const ret = arg0.beginComputePass(arg1);
            return ret;
        },
        __wbg_buffer_a77cc90da4bdb503: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_call_7f2987183bb62793: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_call_d578befcc3145dee: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_copyTextureToBuffer_516f65baac22e0db: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.copyTextureToBuffer(arg1, arg2, arg3);
        }, arguments); },
        __wbg_createBindGroup_3bccbd7517f0708e: function(arg0, arg1) {
            const ret = arg0.createBindGroup(arg1);
            return ret;
        },
        __wbg_createBuffer_24b346170c9f54c8: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBuffer(arg1);
            return ret;
        }, arguments); },
        __wbg_createCommandEncoder_48a406baaa084912: function(arg0, arg1) {
            const ret = arg0.createCommandEncoder(arg1);
            return ret;
        },
        __wbg_createComputePipeline_4efb4ca205a4b557: function(arg0, arg1) {
            const ret = arg0.createComputePipeline(arg1);
            return ret;
        },
        __wbg_createShaderModule_1b0812f3a4503221: function(arg0, arg1) {
            const ret = arg0.createShaderModule(arg1);
            return ret;
        },
        __wbg_createTexture_77337549db437b45: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createTexture(arg1);
            return ret;
        }, arguments); },
        __wbg_createView_13bc5cdadcefa9ec: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createView(arg1);
            return ret;
        }, arguments); },
        __wbg_dispatchWorkgroups_1b750cb68e2eb693: function(arg0, arg1, arg2, arg3) {
            arg0.dispatchWorkgroups(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0);
        },
        __wbg_done_547d467e97529006: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_end_fd65a01a19361ec7: function(arg0) {
            arg0.end();
        },
        __wbg_entries_616b1a459b85be0b: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_finish_2440fb64e53f7d5a: function(arg0, arg1) {
            const ret = arg0.finish(arg1);
            return ret;
        },
        __wbg_finish_4b40810f0b577bc2: function(arg0) {
            const ret = arg0.finish();
            return ret;
        },
        __wbg_getBindGroupLayout_e89dcfe6160ced16: function(arg0, arg1) {
            const ret = arg0.getBindGroupLayout(arg1 >>> 0);
            return ret;
        },
        __wbg_getMappedRange_55878eb97535ca19: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getMappedRange(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_get_4848e350b40afc16: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_ed0642c4b9d31ddf: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_f96702c6245e4ef9: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_unchecked_7d7babe32e9e6a54: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_gpu_bafbc1407fe850fb: function(arg0) {
            const ret = arg0.gpu;
            return ret;
        },
        __wbg_instanceof_ArrayBuffer_ff7c1337a5e3b33a: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuAdapter_aff4b0f95a6c1c3e: function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUAdapter;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Map_a10a2795ef4bfe97: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Map;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_4b8da683deb25d72: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_db61795ad004c139: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isSafeInteger_ea83862ba994770c: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_de403ef31815a3e6: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_label_4b6427d9045e3926: function(arg0, arg1) {
            const ret = arg1.label;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_length_0c32cb8543c8e4c8: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_6e821edde497a532: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_mapAsync_f7fe2e4825742580: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.mapAsync(arg1 >>> 0, arg2, arg3);
            return ret;
        },
        __wbg_navigator_9b09ea705d03d227: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_navigator_af52153252bdf29d: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_4f9fafbb3909af72: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_a560378ea1240b14: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_f3c9df4f38f3f798: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_from_slice_2580ff33d0d10520: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_d85ad974cf8f6f35: function(arg0, arg1) {
            const ret = new Float32Array(getArrayF32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_14d7cc391ce53d2c: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h67baf97aa0fae51e(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_with_byte_offset_and_length_6bfc75833d6170c8: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_next_01132ed6134b8ef5: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_next_b3713ec761a9dbfd: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_prototypesetcall_3e05eb9545565046: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_push_6bdbc990be5ac37b: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_abaf92f0bd4e80a4: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_df5a6dac26d818f3: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queue_3e40156d83b9183e: function(arg0) {
            const ret = arg0.queue;
            return ret;
        },
        __wbg_requestAdapter_245da40985c2fdc5: function(arg0, arg1) {
            const ret = arg0.requestAdapter(arg1);
            return ret;
        },
        __wbg_requestDevice_28434913a23418c4: function(arg0, arg1) {
            const ret = arg0.requestDevice(arg1);
            return ret;
        },
        __wbg_resolve_0a79de24e9d2267b: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_setBindGroup_98f0303f15c3cfb4: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setBindGroup(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
        }, arguments); },
        __wbg_setBindGroup_bc67abae8c962082: function(arg0, arg1, arg2) {
            arg0.setBindGroup(arg1 >>> 0, arg2);
        },
        __wbg_setPipeline_0c34cc40ab8d6499: function(arg0, arg1) {
            arg0.setPipeline(arg1);
        },
        __wbg_set_62f340d5d135b4db: function(arg0, arg1, arg2) {
            arg0.set(arg1, arg2 >>> 0);
        },
        __wbg_set_8ee2d34facb8466e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_array_layer_count_37c76e4cca82351f: function(arg0, arg1) {
            arg0.arrayLayerCount = arg1 >>> 0;
        },
        __wbg_set_aspect_c9292d2a13f954e1: function(arg0, arg1) {
            arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_base_array_layer_6374493b6bc1a0a9: function(arg0, arg1) {
            arg0.baseArrayLayer = arg1 >>> 0;
        },
        __wbg_set_base_mip_level_5a0524f10a35bff6: function(arg0, arg1) {
            arg0.baseMipLevel = arg1 >>> 0;
        },
        __wbg_set_beginning_of_pass_write_index_ac45c363336c24c7: function(arg0, arg1) {
            arg0.beginningOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_binding_0a48264269982c5e: function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        },
        __wbg_set_buffer_3b3e4c4a884d1610: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffer_5c9fd98c06ff0965: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_bytes_per_row_af08702a3d159816: function(arg0, arg1) {
            arg0.bytesPerRow = arg1 >>> 0;
        },
        __wbg_set_code_c616b86ce504e24a: function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_compute_7c274f1347709d07: function(arg0, arg1) {
            arg0.compute = arg1;
        },
        __wbg_set_depth_or_array_layers_e21f6b37c67d8790: function(arg0, arg1) {
            arg0.depthOrArrayLayers = arg1 >>> 0;
        },
        __wbg_set_dimension_117c2064ce996b47: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_dimension_5c6032ac740887c0: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureDimension[arg1];
        },
        __wbg_set_end_of_pass_write_index_c60088bc589e6882: function(arg0, arg1) {
            arg0.endOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_entries_f07df780e3613292: function(arg0, arg1) {
            arg0.entries = arg1;
        },
        __wbg_set_entry_point_aa503b3bb9fed987: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_format_11c7232d92ed699b: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_c38221656906581e: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_height_3ebe4c6ea2510fcc: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_label_392dc66ad76d942d: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_3e06143ad04772ae: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_4f44629bc3c49d4b: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_68e2953cfd33a5a5: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_76c4f74a38ff9bcd: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_79484ec4d6d85bbf: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_861c8e348e26599d: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_d687cfb9a30329c8: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_dcf5143835b5d044: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_e345704005fb385b: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_layout_b9b36c291ee7f2e1: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_cccbb8f794df887c: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_mapped_at_creation_34da9d6bf64b78d6: function(arg0, arg1) {
            arg0.mappedAtCreation = arg1 !== 0;
        },
        __wbg_set_mip_level_1fe1b17b2d4930dc: function(arg0, arg1) {
            arg0.mipLevel = arg1 >>> 0;
        },
        __wbg_set_mip_level_count_ce77bbcd6aa77dfb: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mip_level_count_faa8a47d0fd87c1e: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_module_5f33a55198ad797f: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_offset_1a0f95ffb7dd6f40: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_offset_73eef07e0840c207: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_origin_b315d15931fdd138: function(arg0, arg1) {
            arg0.origin = arg1;
        },
        __wbg_set_power_preference_915480f4b9565dc2: function(arg0, arg1) {
            arg0.powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
        },
        __wbg_set_query_set_0a78c3dcb3650b2b: function(arg0, arg1) {
            arg0.querySet = arg1;
        },
        __wbg_set_required_features_42347bf311233eb6: function(arg0, arg1) {
            arg0.requiredFeatures = arg1;
        },
        __wbg_set_resource_f2d72f59cc9308fc: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_rows_per_image_f3e25334bd0cdec8: function(arg0, arg1) {
            arg0.rowsPerImage = arg1 >>> 0;
        },
        __wbg_set_sample_count_47378e3363905cfe: function(arg0, arg1) {
            arg0.sampleCount = arg1 >>> 0;
        },
        __wbg_set_size_657d97f8d513b5e9: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_6b2fc4a0e39e4d07: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_c78ae8d2e2181815: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_texture_a2c2ca844a3a3014: function(arg0, arg1) {
            arg0.texture = arg1;
        },
        __wbg_set_timestamp_writes_b9e1d87e2f057bd1: function(arg0, arg1) {
            arg0.timestampWrites = arg1;
        },
        __wbg_set_usage_794d488202743c10: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_9aa23fa1e13799a8: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_ba31cd3d9ce977fe: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_view_formats_0a8a8e11cfa73759: function(arg0, arg1) {
            arg0.viewFormats = arg1;
        },
        __wbg_set_width_60b542bb7870a825: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_x_d11527965ec29a57: function(arg0, arg1) {
            arg0.x = arg1 >>> 0;
        },
        __wbg_set_y_55ef7c361345d5fd: function(arg0, arg1) {
            arg0.y = arg1 >>> 0;
        },
        __wbg_set_z_dc148d1e458d403e: function(arg0, arg1) {
            arg0.z = arg1 >>> 0;
        },
        __wbg_static_accessor_GLOBAL_THIS_a1248013d790bf5f: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_f2e0f995a21329ff: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_24f78b6d23f286ea: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_59fd959c540fe405: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stringify_a2c39d991e1bf91d: function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments); },
        __wbg_submit_2521bdd9a232bca7: function(arg0, arg1) {
            arg0.submit(arg1);
        },
        __wbg_then_00eed3ac0b8e82cb: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_479d77cb064907ee: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_a0c8db0381c8994c: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_unmap_815a075fd850cb73: function(arg0) {
            arg0.unmap();
        },
        __wbg_value_7f6052747ccf940f: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_writeBuffer_e8b792fb0962f30d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.writeBuffer(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 607, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__hd28ba1d3b3161cad);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 628, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h8d52734cc70625dc);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
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

function wasm_bindgen__convert__closures_____invoke__hd28ba1d3b3161cad(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__hd28ba1d3b3161cad(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h8d52734cc70625dc(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h8d52734cc70625dc(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h67baf97aa0fae51e(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h67baf97aa0fae51e(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_GpuPowerPreference = ["low-power", "high-performance"];


const __wbindgen_enum_GpuTextureAspect = ["all", "stencil-only", "depth-only"];


const __wbindgen_enum_GpuTextureDimension = ["1d", "2d", "3d"];


const __wbindgen_enum_GpuTextureFormat = ["r8unorm", "r8snorm", "r8uint", "r8sint", "r16uint", "r16sint", "r16float", "rg8unorm", "rg8snorm", "rg8uint", "rg8sint", "r32uint", "r32sint", "r32float", "rg16uint", "rg16sint", "rg16float", "rgba8unorm", "rgba8unorm-srgb", "rgba8snorm", "rgba8uint", "rgba8sint", "bgra8unorm", "bgra8unorm-srgb", "rgb9e5ufloat", "rgb10a2uint", "rgb10a2unorm", "rg11b10ufloat", "rg32uint", "rg32sint", "rg32float", "rgba16uint", "rgba16sint", "rgba16float", "rgba32uint", "rgba32sint", "rgba32float", "stencil8", "depth16unorm", "depth24plus", "depth24plus-stencil8", "depth32float", "depth32float-stencil8", "bc1-rgba-unorm", "bc1-rgba-unorm-srgb", "bc2-rgba-unorm", "bc2-rgba-unorm-srgb", "bc3-rgba-unorm", "bc3-rgba-unorm-srgb", "bc4-r-unorm", "bc4-r-snorm", "bc5-rg-unorm", "bc5-rg-snorm", "bc6h-rgb-ufloat", "bc6h-rgb-float", "bc7-rgba-unorm", "bc7-rgba-unorm-srgb", "etc2-rgb8unorm", "etc2-rgb8unorm-srgb", "etc2-rgb8a1unorm", "etc2-rgb8a1unorm-srgb", "etc2-rgba8unorm", "etc2-rgba8unorm-srgb", "eac-r11unorm", "eac-r11snorm", "eac-rg11unorm", "eac-rg11snorm", "astc-4x4-unorm", "astc-4x4-unorm-srgb", "astc-5x4-unorm", "astc-5x4-unorm-srgb", "astc-5x5-unorm", "astc-5x5-unorm-srgb", "astc-6x5-unorm", "astc-6x5-unorm-srgb", "astc-6x6-unorm", "astc-6x6-unorm-srgb", "astc-8x5-unorm", "astc-8x5-unorm-srgb", "astc-8x6-unorm", "astc-8x6-unorm-srgb", "astc-8x8-unorm", "astc-8x8-unorm-srgb", "astc-10x5-unorm", "astc-10x5-unorm-srgb", "astc-10x6-unorm", "astc-10x6-unorm-srgb", "astc-10x8-unorm", "astc-10x8-unorm-srgb", "astc-10x10-unorm", "astc-10x10-unorm-srgb", "astc-12x10-unorm", "astc-12x10-unorm-srgb", "astc-12x12-unorm", "astc-12x12-unorm-srgb"];


const __wbindgen_enum_GpuTextureViewDimension = ["1d", "2d", "2d-array", "cube", "cube-array", "3d"];
const CopImageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_copimage_free(ptr >>> 0, 1));
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

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

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

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
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

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
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

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
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
    cachedFloat64ArrayMemory0 = null;
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
