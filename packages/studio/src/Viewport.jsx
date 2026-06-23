import { forwardRef, useEffect, useImperativeHandle, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

/**
 * Three.js viewport. Imperative — parent drives it via the ref:
 *   ref.current.setBuffers(geoBuffers)
 *   ref.current.fit()
 */
export const Viewport = forwardRef(function Viewport({ viewMode = 'shaded_wire' }, ref) {
  const mountRef = useRef(null);
  const ctx = useRef(null);

  useEffect(() => {
    const mount = mountRef.current;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio);
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

    const group = new THREE.Group();
    scene.add(group);

    ctx.current = { renderer, scene, camera, controls, group, viewMode, buffers: null };

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
      renderer.dispose();
      mount.removeChild(renderer.domElement);
      ctx.current = null;
    };
  }, []);

  useEffect(() => {
    if (ctx.current) {
      ctx.current.viewMode = viewMode;
      rebuild(ctx.current);
    }
  }, [viewMode]);

  useImperativeHandle(ref, () => ({
    setBuffers(buffers) {
      if (!ctx.current) return;
      ctx.current.buffers = buffers;
      rebuild(ctx.current);
    },
    fit() {
      if (ctx.current) fitCamera(ctx.current);
    },
  }));

  return <div className="pg-viewport" ref={mountRef} />;
});

function clearGroup(group) {
  while (group.children.length) {
    const c = group.children[0];
    c.geometry?.dispose();
    c.material?.dispose();
    group.remove(c);
  }
}

function toBufferGeometry(buffers) {
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(buffers.positions, 3));
  if (buffers.indices?.length) geo.setIndex(buffers.indices);
  if (buffers.colors) geo.setAttribute('color', new THREE.Float32BufferAttribute(buffers.colors, 3));
  if (buffers.normals) geo.setAttribute('normal', new THREE.Float32BufferAttribute(buffers.normals, 3));
  else geo.computeVertexNormals();
  geo.computeBoundingSphere();
  return geo;
}

function rebuild(c) {
  clearGroup(c.group);
  if (!c.buffers) return;
  const showShaded = c.viewMode === 'shaded' || c.viewMode === 'shaded_wire';
  const showWire = c.viewMode === 'wire' || c.viewMode === 'shaded_wire';
  const geo = toBufferGeometry(c.buffers);

  if (showShaded) {
    const hasColors = !!geo.getAttribute('color');
    const mesh = new THREE.Mesh(
      geo,
      new THREE.MeshStandardMaterial({
        color: hasColors ? 0xffffff : 0x4488cc,
        vertexColors: hasColors,
        side: THREE.DoubleSide,
        roughness: 0.55,
        metalness: 0.15,
      }),
    );
    c.group.add(mesh);
  }
  if (showWire) {
    const wireGeo = showShaded ? geo : geo.clone();
    const color = c.viewMode === 'wire' ? 0x88aaff : 0x223355;
    c.group.add(new THREE.LineSegments(new THREE.WireframeGeometry(wireGeo), new THREE.LineBasicMaterial({ color })));
  }
}

function fitCamera(c) {
  const box = new THREE.Box3().setFromObject(c.group);
  if (box.isEmpty()) return;
  const center = box.getCenter(new THREE.Vector3());
  const size = box.getSize(new THREE.Vector3());
  const maxDim = Math.max(size.x, size.y, size.z) || 1;
  const dist = (maxDim / (2 * Math.tan((c.camera.fov * Math.PI) / 360))) * 1.4;
  c.camera.position.copy(center).add(new THREE.Vector3(dist * 0.6, dist * 0.5, dist * 0.7));
  c.controls.target.copy(center);
  c.camera.near = dist * 0.01;
  c.camera.far = dist * 20;
  c.camera.updateProjectionMatrix();
  c.controls.update();
}
