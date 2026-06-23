import { forwardRef, useEffect, useImperativeHandle, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

/**
 * Three.js viewport. Imperative — parent drives it via the ref:
 *   ref.current.setBuffers(geoBuffers)
 *   ref.current.fit()
 *
 * Persistent meshes/materials are reused across cooks; only the BufferGeometry
 * is swapped, so there's no per-cook object/material churn.
 */
export const Viewport = forwardRef(function Viewport({ viewMode = 'shaded_wire' }, ref) {
  const mountRef = useRef(null);
  const ctx = useRef(null);

  useEffect(() => {
    const mount = mountRef.current;
    const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    mount.appendChild(renderer.domElement);
    renderer.domElement.style.display = 'block';

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x181820);

    const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 2000);
    camera.position.set(3, 2.2, 3.5);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = false;

    scene.add(new THREE.AmbientLight(0xffffff, 0.35));
    const dir = new THREE.DirectionalLight(0xffffff, 1.2);
    dir.position.set(5, 8, 6);
    scene.add(dir);
    scene.add(new THREE.DirectionalLight(0x6688bb, 0.4).translateX(-4).translateY(3).translateZ(-5));
    scene.add(new THREE.GridHelper(8, 16, 0x222233, 0x1a1a28));

    // Persistent display objects (geometry swapped per cook).
    const surfaceMat = new THREE.MeshStandardMaterial({
      color: 0x4488cc,
      side: THREE.DoubleSide,
      roughness: 0.55,
      metalness: 0.15,
    });
    const wireMat = new THREE.LineBasicMaterial({ color: 0x223355 });
    const pointsMat = new THREE.PointsMaterial({ size: 0.04, sizeAttenuation: true, color: 0x88aaff });

    const mesh = new THREE.Mesh(undefined, surfaceMat);
    const wire = new THREE.LineSegments(undefined, wireMat);
    const points = new THREE.Points(undefined, pointsMat);
    mesh.visible = wire.visible = points.visible = false;
    mesh.frustumCulled = wire.frustumCulled = points.frustumCulled = false;
    scene.add(mesh, wire, points);

    ctx.current = {
      renderer, scene, camera, controls, viewMode,
      surfaceMat, wireMat, pointsMat, mesh, wire, points, hasTris: false, lastIndices: null,
    };

    const resize = () => {
      const w = mount.clientWidth;
      const h = mount.clientHeight;
      if (w === 0 || h === 0) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    };
    const ro = new ResizeObserver(resize);
    ro.observe(mount);
    resize();

    let raf;
    const loop = () => {
      raf = requestAnimationFrame(loop);
      controls.update();
      renderer.render(scene, camera);
    };
    loop();

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      controls.dispose();
      mesh.geometry?.dispose();
      wire.geometry?.dispose();
      points.geometry?.dispose();
      surfaceMat.dispose();
      wireMat.dispose();
      pointsMat.dispose();
      renderer.dispose();
      mount.removeChild(renderer.domElement);
      ctx.current = null;
    };
  }, []);

  useEffect(() => {
    if (ctx.current) {
      ctx.current.viewMode = viewMode;
      applyVisibility(ctx.current);
    }
  }, [viewMode]);

  useImperativeHandle(ref, () => ({
    setBuffers(buffers) {
      if (ctx.current) setBuffers(ctx.current, buffers);
    },
    fit() {
      if (ctx.current) fitCamera(ctx.current);
    },
  }));

  return <div className="pg-viewport" ref={mountRef} />;
});

function buildGeometry(buffers) {
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(buffers.positions, 3));
  if (buffers.indices?.length) geo.setIndex(new THREE.Uint32BufferAttribute(buffers.indices, 1));
  if (buffers.colors) geo.setAttribute('color', new THREE.Float32BufferAttribute(buffers.colors, 3));
  if (buffers.normals) geo.setAttribute('normal', new THREE.Float32BufferAttribute(buffers.normals, 3));
  else geo.computeVertexNormals();
  geo.computeBoundingSphere();
  return geo;
}

/** Unique edge index (point pairs) from triangle indices, for wireframe. */
function edgeIndex(tri) {
  const seen = new Set();
  const out = [];
  const add = (a, b) => {
    const lo = a < b ? a : b;
    const hi = a < b ? b : a;
    const key = lo + ':' + hi;
    if (!seen.has(key)) {
      seen.add(key);
      out.push(lo, hi);
    }
  };
  for (let i = 0; i + 2 < tri.length; i += 3) {
    add(tri[i], tri[i + 1]);
    add(tri[i + 1], tri[i + 2]);
    add(tri[i + 2], tri[i]);
  }
  return new Uint32Array(out);
}

function sameTopology(a, b) {
  if (!a || !b || a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

/** Copy new vertex data into existing attributes in place (no realloc). The
 *  wireframe shares the mesh's position attribute, so it updates for free. */
function updateInPlace(geo, buffers) {
  geo.attributes.position.array.set(buffers.positions);
  geo.attributes.position.needsUpdate = true;
  if (buffers.normals && geo.attributes.normal) {
    geo.attributes.normal.array.set(buffers.normals);
    geo.attributes.normal.needsUpdate = true;
  } else if (!buffers.normals) {
    geo.computeVertexNormals();
  }
  if (buffers.colors && geo.attributes.color) {
    geo.attributes.color.array.set(buffers.colors);
    geo.attributes.color.needsUpdate = true;
  }
  geo.computeBoundingSphere();
}

function setBuffers(c, buffers) {
  const hasTris = !!(buffers.indices && buffers.indices.length);
  const hasColors = !!buffers.colors;
  c.hasTris = hasTris;

  if (hasTris) {
    const posMatches =
      c.mesh.geometry &&
      c.mesh.geometry.attributes.position.count === buffers.positions.length / 3;

    if (posMatches && sameTopology(buffers.indices, c.lastIndices)) {
      // Same topology → update buffers in place; wireframe shares the position.
      updateInPlace(c.mesh.geometry, buffers);
    } else {
      const g = buildGeometry(buffers);
      c.mesh.geometry?.dispose();
      c.mesh.geometry = g;

      // Wireframe reuses the mesh position attribute + a deduped edge index,
      // so in-place position updates above flow through to it.
      const wg = new THREE.BufferGeometry();
      wg.setAttribute('position', g.attributes.position);
      wg.setIndex(new THREE.Uint32BufferAttribute(edgeIndex(buffers.indices), 1));
      c.wire.geometry?.dispose();
      c.wire.geometry = wg;
      c.lastIndices = buffers.indices;
    }

    c.surfaceMat.color.set(hasColors ? 0xffffff : 0x4488cc);
    c.surfaceMat.vertexColors = hasColors;
    c.surfaceMat.needsUpdate = true;
    c.points.geometry?.dispose();
    c.points.geometry = null;
  } else {
    c.points.geometry?.dispose();
    c.points.geometry = buildGeometry(buffers);
    c.pointsMat.color.set(hasColors ? 0xffffff : 0x88aaff);
    c.pointsMat.vertexColors = hasColors;
    c.pointsMat.needsUpdate = true;
    c.mesh.geometry?.dispose();
    c.mesh.geometry = null;
    c.wire.geometry?.dispose();
    c.wire.geometry = null;
    c.lastIndices = null;
  }
  applyVisibility(c);
}

function applyVisibility(c) {
  if (!c.hasTris) {
    c.points.visible = !!c.points.geometry;
    c.mesh.visible = false;
    c.wire.visible = false;
    return;
  }
  const showShaded = c.viewMode === 'shaded' || c.viewMode === 'shaded_wire';
  const showWire = c.viewMode === 'wire' || c.viewMode === 'shaded_wire';
  c.mesh.visible = showShaded && !!c.mesh.geometry;
  c.wire.visible = showWire && !!c.wire.geometry;
  c.points.visible = false;
  c.wireMat.color.set(c.viewMode === 'wire' ? 0x88aaff : 0x223355);
}

function fitCamera(c) {
  const target = c.mesh.geometry || c.points.geometry;
  if (!target?.boundingSphere) return;
  const { center, radius } = target.boundingSphere;
  const maxDim = radius * 2 || 1;
  const dist = (maxDim / (2 * Math.tan((c.camera.fov * Math.PI) / 360))) * 1.4;
  c.camera.position.copy(center).add(new THREE.Vector3(dist * 0.6, dist * 0.5, dist * 0.7));
  c.controls.target.copy(center);
  c.camera.near = Math.max(dist * 0.01, 0.001);
  c.camera.far = dist * 20;
  c.camera.updateProjectionMatrix();
  c.controls.update();
}
