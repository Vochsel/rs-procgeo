// Source of truth for Monaco autocomplete in the web playground.
// Keep this file aligned with the generated WASM surface in web/wasm/procgeo_wasm.d.ts.
// scripts/validate-web-editor-types.mjs enforces that the runtime API is fully covered here.

type ProcGeoVec2 = [number, number];
type ProcGeoVec3 = [number, number, number];
type ProcGeoRgb = [number, number, number];
type ProcGeoRgba = [number, number, number, number];

type ProcGeoAttribClass = "Point" | "Vertex" | "Primitive" | "Detail";
type ProcGeoAttribClassRef =
    | ProcGeoAttribClass
    | "point"
    | "vertex"
    | "primitive"
    | "detail"
    | "prim";
type ProcGeoAttribType = "Float" | "Int" | "Vector3" | "String";
type ProcGeoGroupType = "points" | "primitives";
type ProcGeoJsonParams = Record<string, unknown>;

type ProcGeoSubdivideMode = "linear" | "catmullClark";
type ProcGeoQuadRemeshTargetMode = "faceCount" | "vertexCount" | "edgeLength";
type ProcGeoQuadRemeshMode = "intrinsic" | "extrinsic";
type ProcGeoBendMode = "angle" | "direction";
type ProcGeoTaperMode = "linear" | "smooth";
type ProcGeoBooleanOperation =
    | "union"
    | "intersect"
    | "subtract"
    | "shatter"
    | "seam"
    | "detect";
type ProcGeoBooleanTreatAs = "solid" | "surface";
type ProcGeoMetaballKernel = "wyvill" | "blinn" | "hart";
type ProcGeoGroupCreateMode = "range" | "boundingBox" | "normal";
type ProcGeoGroupCombineOperation = "union" | "intersect" | "subtract";
type ProcGeoPolyFillMode = "single" | "fan";
type ProcGeoAttribPromoteMethod = "average" | "first" | "last" | "min" | "max";
type ProcGeoAttribRandomizeDistribution = "Uniform" | "Gaussian" | "Bernoulli";
type ProcGeoAttribRandomizeOperation = "Set" | "Add" | "Multiply";
type ProcGeoAttribNoiseType = "perlin" | "simplex" | "worley" | "worleyF2F1";
type ProcGeoAttribNoiseOperation =
    | "setInitial"
    | "set"
    | "add"
    | "subtract"
    | "multiply"
    | "min"
    | "max";
type ProcGeoAttribNoiseFractal = "none" | "standard" | "terrain";
type ProcGeoAttribNoiseRange = "positive" | "zeroCentered" | "minMax";
type ProcGeoAttribSortOrder = "Ascending" | "Descending";
type ProcGeoCopNoiseType = "perlin" | "simplex" | "worley";
type ProcGeoCopRampType = "linear" | "radial" | "box" | "diagonal";
type ProcGeoCopBlurType = "gaussian" | "box";
type ProcGeoCopMirrorAxis = "x" | "y";
type ProcGeoCopChannel = "r" | "g" | "b" | "a" | "one" | "zero";
type ProcGeoCopFilter = "nearest" | "bilinear";
type ProcGeoCopCompositeOperation =
    | "over"
    | "add"
    | "multiply"
    | "screen"
    | "subtract"
    | "difference"
    | "min"
    | "max";
type ProcGeoCopShaderLanguage = "wgsl" | "glsl";

interface ProcGeoBoundingBox {
    min: Float32Array;
    max: Float32Array;
}

interface ProcGeoGeometry {
    /** Release the underlying WASM allocation. */
    free(): void;
    /** Add a point at position [x, y, z]. Returns the point index. */
    addPoint(x: number, y: number, z: number): number;
    /** Set the position of an existing point. */
    setPointPos(index: number, x: number, y: number, z: number): void;
    /** Create a closed face (polygon) from point indices. Returns the primitive index. */
    addFace(pointIndices: Uint32Array | number[]): number;
    /** Create an open polyline from point indices. Returns the primitive index. */
    addPolyline(pointIndices: Uint32Array | number[]): number;
    /** Number of points in the geometry. */
    readonly numPoints: number;
    /** Number of primitives (faces/curves) in the geometry. */
    readonly numPrims: number;
    /** Number of vertices in the geometry. */
    readonly numVertices: number;
    /** Get position of a point by index. */
    pointPos(index: number): Float32Array;
    /** Get the axis-aligned bounding box. */
    boundingBox(): ProcGeoBoundingBox;
    /** Get all positions as a flat Float32Array [x0,y0,z0, x1,y1,z1, ...]. */
    getPositions(): Float32Array;
    /** Get triangle indices as a flat Uint32Array. */
    getTriangleIndices(): Uint32Array;
    /** Get normals as a flat Float32Array (if "N" attribute exists). */
    getNormals(): Float32Array | undefined;
    /** Get vertex colors as a flat Float32Array (if "Cd" attribute exists). */
    getColors(): Float32Array | undefined;
    /** List attribute names for a class ("point", "vertex", "primitive", "detail"). */
    attribNames(klass: ProcGeoAttribClassRef): string[];
    /** Get the type name of an attribute ("Float", "Int", "Vector3", etc.). */
    attribType(klass: ProcGeoAttribClassRef, name: string): string | undefined;
    /** Get the component count of an attribute (1 for float, 3 for vec3, etc.). */
    attribSize(klass: ProcGeoAttribClassRef, name: string): number | undefined;
    /** Get all numeric values of an attribute as a flat Float64Array. */
    attribData(klass: ProcGeoAttribClassRef, name: string): Float64Array | undefined;
    /** Get all string values of an attribute. */
    attribDataString(klass: ProcGeoAttribClassRef, name: string): string[] | undefined;
    /** Get the point indices for a specific primitive. */
    primPointIndices(primIndex: number): Uint32Array;
    /** Return whether a primitive is closed. */
    primIsClosed(primIndex: number): boolean;
    /** Get the number of vertices in a specific primitive. */
    primVertexCount(primIndex: number): number;
    /** Get which point a vertex maps to. */
    vertexPoint(vertexIndex: number): number;
    /** Export geometry as OBJ string. */
    toObj(): string;
    /** Export geometry as GLB binary. */
    toGlb(): Uint8Array;
}

interface ProcGeoGeometryConstructor {
    new(): ProcGeoGeometry;
    readonly prototype: ProcGeoGeometry;
}

interface ProcGeoCopImage {
    /** Release the underlying WASM allocation. */
    free(): void;
    /** Read pixel data back from GPU as Float32Array (RGBA per pixel). */
    getPixels(): Promise<Float32Array>;
    /** Image width in pixels. */
    readonly width: number;
    /** Image height in pixels. */
    readonly height: number;
}

interface ProcGeoCopImageConstructor {
    readonly prototype: ProcGeoCopImage;
}

interface ProcGeoCreateBoxParams {
    size?: ProcGeoVec3;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateGridParams {
    rows?: number;
    cols?: number;
    sizeX?: number;
    sizeY?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateSphereParams {
    radius?: number;
    rows?: number;
    cols?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateLineParams {
    origin?: ProcGeoVec3;
    direction?: ProcGeoVec3;
    length?: number;
    points?: number;
}

interface ProcGeoCreateSpiralParams {
    startRadius?: number;
    endRadius?: number;
    height?: number;
    turns?: number;
    points?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateHelixParams {
    radius?: number;
    height?: number;
    turns?: number;
    points?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateCircleParams {
    radius?: number;
    divisions?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateTubeParams {
    radiusBottom?: number;
    radiusTop?: number;
    height?: number;
    cols?: number;
    rows?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateTorusParams {
    radiusOuter?: number;
    radiusInner?: number;
    rows?: number;
    cols?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateIcosphereParams {
    radius?: number;
    subdivisions?: number;
    center?: ProcGeoVec3;
}

interface ProcGeoCreateTeapotParams {
    size?: ProcGeoVec3;
    center?: ProcGeoVec3;
    resolution?: number;
}

interface ProcGeoTransformParams {
    translate?: ProcGeoVec3;
    rotate?: ProcGeoVec3;
    scale?: ProcGeoVec3;
    pivot?: ProcGeoVec3;
}

interface ProcGeoScatterParams {
    count?: number;
    seed?: number;
}

interface ProcGeoSubdivideParams {
    depth?: number;
    mode?: ProcGeoSubdivideMode;
}

interface ProcGeoPolyExtrudeParams {
    distance?: number;
    inset?: number;
    outputFront?: boolean;
    outputSide?: boolean;
}

interface ProcGeoSmoothParams {
    iterations?: number;
    strength?: number;
}

interface ProcGeoClipParams {
    origin?: ProcGeoVec3;
    normal?: ProcGeoVec3;
    keepAbove?: boolean;
    createCap?: boolean;
}

interface ProcGeoPolyBevelParams {
    offset?: number;
    divisions?: number;
}

interface ProcGeoPolyWireParams {
    radius?: number;
    divisions?: number;
}

interface ProcGeoPolyReduceParams {
    targetPercent?: number;
    preserveBoundaries?: boolean;
}

interface ProcGeoPolyFillParams {
    mode?: ProcGeoPolyFillMode;
    smooth?: number;
}

interface ProcGeoQuadRemeshParams {
    targetMode?: ProcGeoQuadRemeshTargetMode;
    targetCount?: number;
    targetEdgeLength?: number;
    seed?: number;
    mode?: ProcGeoQuadRemeshMode;
}

interface ProcGeoQuadWildParams {
    sharpAngle?: number;
    curvatureWeight?: number;
    smoothIterations?: number;
    scaleFactor?: number;
    alpha?: number;
    postSmoothIterations?: number;
}

interface ProcGeoFuseParams {
    distance?: number;
}

interface ProcGeoSortParams {
    seed?: number;
}

interface ProcGeoResampleParams {
    length?: number;
    maxSegments?: number;
}

interface ProcGeoConnectivityParams {
    attribName?: string;
}

interface ProcGeoColorParams {
    color?: ProcGeoRgb;
}

interface ProcGeoBendParams {
    group?: string;
    maskAttrib?: string;
    enableDeformation?: boolean;
    limitToCaptureRegion?: boolean;
    deformBothDirections?: boolean;
    bendEnable?: boolean;
    bendMode?: ProcGeoBendMode;
    bendAngle?: number;
    bendGoalDirection?: ProcGeoVec3;
    twistEnable?: boolean;
    twistAngle?: number;
    twistContinuousBoth?: boolean;
    lengthScaleEnable?: boolean;
    lengthScale?: number;
    preserveVolume?: boolean;
    taperEnable?: boolean;
    taperAlongX?: boolean;
    taperAlongY?: boolean;
    taperMode?: ProcGeoTaperMode;
    taperValue?: number;
    squish?: number;
    squishPivot?: number;
    upVector?: ProcGeoVec3;
    upVectorAngle?: number;
    captureOrigin?: ProcGeoVec3;
    captureDirection?: ProcGeoVec3;
    captureLength?: number;
}

interface ProcGeoPointDeformParams {
    radius?: number;
    minPoints?: number;
    maxPoints?: number;
    rigidProjection?: boolean;
    mask?: number;
}

interface ProcGeoBooleanParams {
    operation?: ProcGeoBooleanOperation;
    treatAAs?: ProcGeoBooleanTreatAs;
    treatBAs?: ProcGeoBooleanTreatAs;
    collapseTinyEdges?: boolean;
}

interface ProcGeoVoronoiFractureParams {
    cutPlaneOffset?: number;
    createInsideFaces?: boolean;
}

interface ProcGeoMetaballBall {
    center?: ProcGeoVec3;
    radius?: number;
    weight?: number;
}

interface ProcGeoCreateMetaballParams {
    balls?: ProcGeoMetaballBall[];
    kernel?: ProcGeoMetaballKernel;
    threshold?: number;
    resolution?: number;
    padding?: number;
}

interface ProcGeoRevolveParams {
    origin?: ProcGeoVec3;
    axis?: ProcGeoVec3;
    divisions?: number;
    startAngle?: number;
    endAngle?: number;
    endCaps?: boolean;
}

interface ProcGeoGroupCreateParams {
    name?: string;
    groupType?: ProcGeoGroupType;
    mode?: ProcGeoGroupCreateMode;
    rangeStart?: number;
    rangeEnd?: number;
    bboxMin?: ProcGeoVec3;
    bboxMax?: ProcGeoVec3;
    normalDirection?: ProcGeoVec3;
    normalAngle?: number;
}

interface ProcGeoGroupCombineParams {
    nameA?: string;
    nameB?: string;
    result?: string;
    operation?: ProcGeoGroupCombineOperation;
    groupType?: ProcGeoGroupType;
}

interface ProcGeoBlastParams {
    groupName?: string;
    entity?: ProcGeoGroupType;
    negate?: boolean;
}

interface ProcGeoDeleteSopParams {
    entity?: ProcGeoGroupType;
    rangeStart?: number;
    rangeEnd?: number;
}

interface ProcGeoAttribCreateParams {
    name?: string;
    class?: ProcGeoAttribClass;
    attribType?: ProcGeoAttribType;
    valueInt?: number;
    valueFloat?: number;
    valueVector3?: ProcGeoVec3;
    valueString?: string;
}

interface ProcGeoAttribDeleteParams {
    name?: string;
    class?: ProcGeoAttribClass;
}

interface ProcGeoAttribRenameParams {
    fromName?: string;
    toName?: string;
    class?: ProcGeoAttribClass;
}

interface ProcGeoAttribPromoteParams {
    name?: string;
    fromClass?: ProcGeoAttribClass;
    toClass?: ProcGeoAttribClass;
    method?: ProcGeoAttribPromoteMethod;
    deleteOriginal?: boolean;
}

interface ProcGeoAttribTransferParams {
    attribName?: string;
    class?: ProcGeoAttribClass;
    attribType?: ProcGeoAttribType;
    maxSamples?: number;
    distanceThreshold?: number;
}

interface ProcGeoAttribCopyParams {
    attribName?: string;
    class?: ProcGeoAttribClass;
    attribType?: ProcGeoAttribType;
    newName?: string;
}

interface ProcGeoAttribRandomizeParams {
    attribName?: string;
    class?: ProcGeoAttribClass;
    attribType?: ProcGeoAttribType;
    distribution?: ProcGeoAttribRandomizeDistribution;
    operation?: ProcGeoAttribRandomizeOperation;
    seed?: number;
    minValue?: number;
    maxValue?: number;
    dimensions?: number;
    globalScale?: number;
}

interface ProcGeoAttribSortParams {
    attribName?: string;
    class?: ProcGeoAttribClass;
    attribType?: ProcGeoAttribType;
    order?: ProcGeoAttribSortOrder;
    component?: number;
}

interface ProcGeoAttribBlurParams {
    attribName?: string;
    attribType?: ProcGeoAttribType;
    iterations?: number;
    stepSize?: number;
}

interface ProcGeoAttribFillParams {
    attribName?: string;
    attribType?: ProcGeoAttribType;
    boundaryGroup?: string;
    iterations?: number;
    stepSize?: number;
}

interface ProcGeoAttribNoiseParams {
    attribName?: string;
    noiseType?: ProcGeoAttribNoiseType;
    operation?: ProcGeoAttribNoiseOperation;
    elementSize?: number;
    amplitude?: number;
    seed?: number;
    dimensions?: number;
    fractal?: ProcGeoAttribNoiseFractal;
    octaves?: number;
    lacunarity?: number;
    roughness?: number;
    range?: ProcGeoAttribNoiseRange;
    minValue?: number;
    maxValue?: number;
    offset?: ProcGeoVec3;
    gain?: number;
    bias?: number;
}

interface ProcGeoEnumerateAttribParams {
    name?: string;
    start?: number;
}

interface ProcGeoMeasureParams {
    attribName?: string;
}

interface ProcGeoCopConstantParams {
    color?: ProcGeoRgb | ProcGeoRgba | number[];
    width?: number;
    height?: number;
}

interface ProcGeoCopCheckerboardParams {
    colorA?: ProcGeoRgb | ProcGeoRgba | number[];
    colorB?: ProcGeoRgb | ProcGeoRgba | number[];
    frequency?: ProcGeoVec2 | number[];
    width?: number;
    height?: number;
}

interface ProcGeoCopNoiseParams {
    noiseType?: ProcGeoCopNoiseType;
    frequency?: number;
    octaves?: number;
    lacunarity?: number;
    gain?: number;
    amplitude?: number;
    offset?: ProcGeoVec2 | number[];
    seed?: number;
    width?: number;
    height?: number;
}

interface ProcGeoCopRampStop {
    position: number;
    color: ProcGeoRgb | ProcGeoRgba | number[];
}

interface ProcGeoCopRampParams {
    rampType?: ProcGeoCopRampType;
    stops?: ProcGeoCopRampStop[];
    width?: number;
    height?: number;
}

interface ProcGeoCopLoadImageParams {
    path?: string;
}

interface ProcGeoCopBlurParams {
    blurType?: ProcGeoCopBlurType;
    radiusX?: number;
    radiusY?: number;
}

interface ProcGeoCopFlipParams {
    horizontal?: boolean;
    vertical?: boolean;
}

interface ProcGeoCopMirrorParams {
    axis?: ProcGeoCopMirrorAxis;
    offset?: number;
}

interface ProcGeoCopChannelSwapParams {
    r?: ProcGeoCopChannel;
    g?: ProcGeoCopChannel;
    b?: ProcGeoCopChannel;
    a?: ProcGeoCopChannel;
}

interface ProcGeoCopResizeParams {
    width?: number;
    height?: number;
    filter?: ProcGeoCopFilter;
}

interface ProcGeoCopRotateParams {
    angle?: number;
    center?: ProcGeoVec2 | number[];
    filter?: ProcGeoCopFilter;
}

interface ProcGeoCopSwirlParams {
    center?: ProcGeoVec2 | number[];
    angle?: number;
    radius?: number;
}

interface ProcGeoCopCompositeParams {
    operation?: ProcGeoCopCompositeOperation;
    mix?: number;
}

interface ProcGeoCopCustomShaderParams {
    source?: string;
    language?: ProcGeoCopShaderLanguage;
    width?: number;
    height?: number;
}

interface ProcGeoModule {
    readonly Geometry: ProcGeoGeometryConstructor;
    readonly CopImage: ProcGeoCopImageConstructor;

    /** Initialize the GPU context for COP image processing. */
    initCopGpu(): Promise<void>;

    // Creation
    createBox(params?: ProcGeoCreateBoxParams): ProcGeoGeometry;
    createGrid(params?: ProcGeoCreateGridParams): ProcGeoGeometry;
    createSphere(params?: ProcGeoCreateSphereParams): ProcGeoGeometry;
    createLine(params?: ProcGeoCreateLineParams): ProcGeoGeometry;
    createSpiral(params?: ProcGeoCreateSpiralParams): ProcGeoGeometry;
    createHelix(params?: ProcGeoCreateHelixParams): ProcGeoGeometry;
    createCircle(params?: ProcGeoCreateCircleParams): ProcGeoGeometry;
    createTube(params?: ProcGeoCreateTubeParams): ProcGeoGeometry;
    createTorus(params?: ProcGeoCreateTorusParams): ProcGeoGeometry;
    createIcosphere(params?: ProcGeoCreateIcosphereParams): ProcGeoGeometry;
    createTeapot(params?: ProcGeoCreateTeapotParams): ProcGeoGeometry;
    createMetaball(params?: ProcGeoCreateMetaballParams): ProcGeoGeometry;
    revolve(geo: ProcGeoGeometry, params?: ProcGeoRevolveParams): ProcGeoGeometry;

    // Transform / deform
    transform(geo: ProcGeoGeometry, params?: ProcGeoTransformParams): ProcGeoGeometry;
    computeNormals(geo: ProcGeoGeometry): ProcGeoGeometry;
    bend(geo: ProcGeoGeometry, params?: ProcGeoBendParams): ProcGeoGeometry;
    pointDeform(
        geo: ProcGeoGeometry,
        restLattice: ProcGeoGeometry,
        deformedLattice: ProcGeoGeometry,
        params?: ProcGeoPointDeformParams
    ): ProcGeoGeometry;

    // Copy / scatter / merge
    scatter(geo: ProcGeoGeometry, params?: ProcGeoScatterParams): ProcGeoGeometry;
    copyToPoints(source: ProcGeoGeometry, target: ProcGeoGeometry): ProcGeoGeometry;
    merge(a: ProcGeoGeometry, b: ProcGeoGeometry): ProcGeoGeometry;

    // Reshape / remesh
    subdivide(geo: ProcGeoGeometry, params?: ProcGeoSubdivideParams): ProcGeoGeometry;
    polyExtrude(geo: ProcGeoGeometry, params?: ProcGeoPolyExtrudeParams): ProcGeoGeometry;
    smooth(geo: ProcGeoGeometry, params?: ProcGeoSmoothParams): ProcGeoGeometry;
    clip(geo: ProcGeoGeometry, params?: ProcGeoClipParams): ProcGeoGeometry;
    polyBevel(geo: ProcGeoGeometry, params?: ProcGeoPolyBevelParams): ProcGeoGeometry;
    polyWire(geo: ProcGeoGeometry, params?: ProcGeoPolyWireParams): ProcGeoGeometry;
    polyReduce(geo: ProcGeoGeometry, params?: ProcGeoPolyReduceParams): ProcGeoGeometry;
    polyFill(geo: ProcGeoGeometry, params?: ProcGeoPolyFillParams): ProcGeoGeometry;
    quadRemesh(geo: ProcGeoGeometry, params?: ProcGeoQuadRemeshParams): ProcGeoGeometry;
    quadWild(geo: ProcGeoGeometry, params?: ProcGeoQuadWildParams): ProcGeoGeometry;

    // Topology / boolean / fracture
    reverse(geo: ProcGeoGeometry): ProcGeoGeometry;
    fuse(geo: ProcGeoGeometry, params?: ProcGeoFuseParams): ProcGeoGeometry;
    sort(geo: ProcGeoGeometry, params?: ProcGeoSortParams): ProcGeoGeometry;
    resample(geo: ProcGeoGeometry, params?: ProcGeoResampleParams): ProcGeoGeometry;
    connectivity(geo: ProcGeoGeometry, params?: ProcGeoConnectivityParams): ProcGeoGeometry;
    booleanOp(
        a: ProcGeoGeometry,
        b: ProcGeoGeometry,
        params?: ProcGeoBooleanParams
    ): ProcGeoGeometry;
    voronoiFracture(
        geo: ProcGeoGeometry,
        points: ProcGeoGeometry,
        params?: ProcGeoVoronoiFractureParams
    ): ProcGeoGeometry;

    // Color / groups / delete / attributes / utility
    color(geo: ProcGeoGeometry, params?: ProcGeoColorParams): ProcGeoGeometry;
    groupCreate(geo: ProcGeoGeometry, params?: ProcGeoGroupCreateParams): ProcGeoGeometry;
    groupCombine(geo: ProcGeoGeometry, params?: ProcGeoGroupCombineParams): ProcGeoGeometry;
    blast(geo: ProcGeoGeometry, params?: ProcGeoBlastParams): ProcGeoGeometry;
    deleteSop(geo: ProcGeoGeometry, params?: ProcGeoDeleteSopParams): ProcGeoGeometry;
    attribCreate(geo: ProcGeoGeometry, params?: ProcGeoAttribCreateParams): ProcGeoGeometry;
    attribDelete(geo: ProcGeoGeometry, params?: ProcGeoAttribDeleteParams): ProcGeoGeometry;
    attribRename(geo: ProcGeoGeometry, params?: ProcGeoAttribRenameParams): ProcGeoGeometry;
    attribPromote(geo: ProcGeoGeometry, params?: ProcGeoAttribPromoteParams): ProcGeoGeometry;
    attribTransfer(
        dest: ProcGeoGeometry,
        source: ProcGeoGeometry,
        params?: ProcGeoAttribTransferParams
    ): ProcGeoGeometry;
    attribCopy(
        dest: ProcGeoGeometry,
        source?: ProcGeoGeometry | null,
        params?: ProcGeoAttribCopyParams
    ): ProcGeoGeometry;
    attribRandomize(geo: ProcGeoGeometry, params?: ProcGeoAttribRandomizeParams): ProcGeoGeometry;
    attribSort(geo: ProcGeoGeometry, params?: ProcGeoAttribSortParams): ProcGeoGeometry;
    attribBlur(geo: ProcGeoGeometry, params?: ProcGeoAttribBlurParams): ProcGeoGeometry;
    attribFill(geo: ProcGeoGeometry, params?: ProcGeoAttribFillParams): ProcGeoGeometry;
    attribNoise(geo: ProcGeoGeometry, params?: ProcGeoAttribNoiseParams): ProcGeoGeometry;
    enumerateAttrib(geo: ProcGeoGeometry, params?: ProcGeoEnumerateAttribParams): ProcGeoGeometry;
    measure(geo: ProcGeoGeometry, params?: ProcGeoMeasureParams): ProcGeoGeometry;

    // Registry-based SOP dispatch
    executeSop(name: string, geo: ProcGeoGeometry, params?: ProcGeoJsonParams): ProcGeoGeometry;
    executeSopCreate(name: string, params?: ProcGeoJsonParams): ProcGeoGeometry;
    listSops(): string[];

    // Registry-based COP dispatch
    executeCopCreate(name: string, params?: ProcGeoJsonParams): ProcGeoCopImage;
    executeCop(name: string, image: ProcGeoCopImage, params?: ProcGeoJsonParams): ProcGeoCopImage;
    executeCopComposite(
        name: string,
        imageA: ProcGeoCopImage,
        imageB: ProcGeoCopImage,
        params?: ProcGeoJsonParams
    ): ProcGeoCopImage;
    listCops(): string[];

    // Typed COP functions
    copConstant(params?: ProcGeoCopConstantParams): ProcGeoCopImage;
    copCheckerboard(params?: ProcGeoCopCheckerboardParams): ProcGeoCopImage;
    copNoise(params?: ProcGeoCopNoiseParams): ProcGeoCopImage;
    copRamp(params?: ProcGeoCopRampParams): ProcGeoCopImage;
    copLoadImage(params?: ProcGeoCopLoadImageParams): ProcGeoCopImage;
    copBlur(image: ProcGeoCopImage, params?: ProcGeoCopBlurParams): ProcGeoCopImage;
    copFlip(image: ProcGeoCopImage, params?: ProcGeoCopFlipParams): ProcGeoCopImage;
    copMirror(image: ProcGeoCopImage, params?: ProcGeoCopMirrorParams): ProcGeoCopImage;
    copChannelSwap(image: ProcGeoCopImage, params?: ProcGeoCopChannelSwapParams): ProcGeoCopImage;
    copResize(image: ProcGeoCopImage, params?: ProcGeoCopResizeParams): ProcGeoCopImage;
    copRotate(image: ProcGeoCopImage, params?: ProcGeoCopRotateParams): ProcGeoCopImage;
    copSwirl(image: ProcGeoCopImage, params?: ProcGeoCopSwirlParams): ProcGeoCopImage;
    copComposite(
        a: ProcGeoCopImage,
        b: ProcGeoCopImage,
        params?: ProcGeoCopCompositeParams
    ): ProcGeoCopImage;
    copCustomShader(
        inputA?: ProcGeoCopImage | null,
        inputB?: ProcGeoCopImage | null,
        params?: ProcGeoCopCustomShaderParams
    ): ProcGeoCopImage;
}

declare const pg: ProcGeoModule;
