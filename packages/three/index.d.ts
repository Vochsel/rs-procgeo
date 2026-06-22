import * as THREE from 'three';

/** Any object that implements the ProcGeo Geometry interface. */
export interface ProcGeoGeometry {
  getPositions(): Float32Array;
  getTriangleIndices(): Uint32Array;
  getNormals?(): Float32Array | undefined;
  getColors?(): Float32Array | undefined;
  numPoints: number;
  numPrims: number;
}

export interface BufferGeometryOptions {
  computeNormals?: boolean;
}

export interface MeshOptions {
  material?: THREE.Material;
  wireframe?: boolean;
  flat?: boolean;
  color?: number | string;
}

export interface WireframeOptions {
  color?: number | string;
  linewidth?: number;
}

export interface PointCloudOptions {
  color?: number | string;
  size?: number;
}

export interface EdgeOptions {
  thresholdAngle?: number;
  color?: number | string;
}

export interface SceneOptions {
  background?: number | string;
  antialias?: boolean;
}

export interface SceneResult {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  animate: (callback?: () => void) => void;
}

export function toBufferGeometry(geo: ProcGeoGeometry, options?: BufferGeometryOptions): THREE.BufferGeometry;
export function toMesh(geo: ProcGeoGeometry, options?: MeshOptions): THREE.Mesh;
export function toWireframe(geo: ProcGeoGeometry, options?: WireframeOptions): THREE.LineSegments;
export function toPointCloud(geo: ProcGeoGeometry, options?: PointCloudOptions): THREE.Points;
export function toEdges(geo: ProcGeoGeometry, options?: EdgeOptions): THREE.LineSegments;
export function createScene(container: HTMLCanvasElement | HTMLElement, options?: SceneOptions): SceneResult;
