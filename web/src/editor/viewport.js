// ─────────────────────────────────────────────────────────────────────────────
// Viewport — Three.js rendering of a procgeo Geometry result.
// Supports shaded / wireframe / shaded+wire display, true polygon wireframes
// (built from primitive edges, not triangulated), and camera framing.
// ─────────────────────────────────────────────────────────────────────────────

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

export class Viewport {
    constructor(canvas, container) {
        this.canvas = canvas;
        this.container = container;
        this.viewMode = 'shaded_wire';
        this.currentGeo = null;

        this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
        this.renderer.setPixelRatio(window.devicePixelRatio);
        this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
        this.renderer.toneMappingExposure = 1.1;

        this.scene = new THREE.Scene();
        this.scene.background = new THREE.Color(0x1b1b22);

        this.camera = new THREE.PerspectiveCamera(45, 1, 0.01, 1000);
        this.camera.position.set(3, 2.2, 3.5);

        this.controls = new OrbitControls(this.camera, canvas);
        this.controls.enableDamping = false;

        this.scene.add(new THREE.AmbientLight(0xffffff, 0.35));
        const dir = new THREE.DirectionalLight(0xffffff, 1.2);
        dir.position.set(5, 8, 6);
        this.scene.add(dir);
        this.scene.add(new THREE.DirectionalLight(0x6688bb, 0.4).translateX(-4).translateY(3).translateZ(-5));

        this.grid = new THREE.GridHelper(8, 16, 0x2a2a3a, 0x202028);
        this.scene.add(this.grid);

        this.meshGroup = new THREE.Group();
        this.scene.add(this.meshGroup);

        this._animate = this._animate.bind(this);
        requestAnimationFrame(this._animate);
    }

    _animate() {
        requestAnimationFrame(this._animate);
        this.controls.update();
        this.renderer.render(this.scene, this.camera);
    }

    resize() {
        const w = this.container.clientWidth;
        const h = this.container.clientHeight;
        if (w > 0 && h > 0) {
            this.camera.aspect = w / h;
            this.camera.updateProjectionMatrix();
            this.renderer.setSize(w, h, false);
        }
    }

    setViewMode(mode) {
        this.viewMode = mode;
        this._rebuild();
    }

    setGeometry(geo) {
        this.currentGeo = geo;
        this._rebuild();
    }

    clear() {
        this.currentGeo = null;
        this._clearGroup();
    }

    _clearGroup() {
        while (this.meshGroup.children.length) {
            const c = this.meshGroup.children[0];
            c.geometry?.dispose();
            c.material?.dispose();
            this.meshGroup.remove(c);
        }
    }

    _rebuild() {
        this._clearGroup();
        const geo = this.currentGeo;
        if (!geo) return;

        const showShaded = this.viewMode === 'shaded' || this.viewMode === 'shaded_wire';
        const showWire = this.viewMode === 'wire' || this.viewMode === 'shaded_wire';

        if (showShaded) {
            const bufGeo = this._toBufferGeometry(geo);
            const hasColors = !!bufGeo.getAttribute('color');
            const mesh = new THREE.Mesh(bufGeo, new THREE.MeshStandardMaterial({
                color: hasColors ? 0xffffff : 0x4488cc,
                vertexColors: hasColors,
                side: THREE.DoubleSide,
                roughness: 0.55,
                metalness: 0.15,
            }));
            this.meshGroup.add(mesh);
        }

        if (showWire) {
            const wireColor = this.viewMode === 'wire' ? 0x88aaff : 0x223355;
            this.meshGroup.add(this._buildTrueWireframe(geo, wireColor));
        }
    }

    _toBufferGeometry(geo) {
        const bufGeo = new THREE.BufferGeometry();
        bufGeo.setAttribute('position', new THREE.BufferAttribute(geo.getPositions(), 3));
        bufGeo.setIndex(new THREE.BufferAttribute(geo.getTriangleIndices(), 1));
        const normals = geo.getNormals?.();
        if (normals) bufGeo.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
        else bufGeo.computeVertexNormals();
        const colors = geo.getColors?.();
        if (colors) bufGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
        bufGeo.computeBoundingSphere();
        return bufGeo;
    }

    // Build LineSegments from original polygon edges (not triangulated).
    _buildTrueWireframe(geo, color) {
        const positions = geo.getPositions();
        const numPrims = geo.numPrims;
        const edgeSet = new Set();
        const edgePairs = [];

        for (let p = 0; p < numPrims; p++) {
            const pts = geo.primPointIndices(p);
            const n = pts.length;
            if (n < 2) continue;
            const isClosed = geo.primIsClosed(p);
            const edgeCount = isClosed ? n : n - 1;
            for (let i = 0; i < edgeCount; i++) {
                const a = pts[i];
                const b = pts[(i + 1) % n];
                const key = Math.min(a, b) * 1000000 + Math.max(a, b);
                if (!edgeSet.has(key)) {
                    edgeSet.add(key);
                    edgePairs.push(a, b);
                }
            }
        }

        const linePositions = new Float32Array(edgePairs.length * 3);
        for (let i = 0; i < edgePairs.length; i++) {
            const idx = edgePairs[i];
            linePositions[i * 3] = positions[idx * 3];
            linePositions[i * 3 + 1] = positions[idx * 3 + 1];
            linePositions[i * 3 + 2] = positions[idx * 3 + 2];
        }
        const lineGeo = new THREE.BufferGeometry();
        lineGeo.setAttribute('position', new THREE.BufferAttribute(linePositions, 3));
        return new THREE.LineSegments(lineGeo, new THREE.LineBasicMaterial({ color }));
    }

    frame() {
        const box = new THREE.Box3().setFromObject(this.meshGroup);
        if (box.isEmpty()) return;
        const center = box.getCenter(new THREE.Vector3());
        const size = box.getSize(new THREE.Vector3());
        const maxDim = Math.max(size.x, size.y, size.z) || 1;
        const dist = (maxDim / (2 * Math.tan((this.camera.fov * Math.PI) / 360))) * 1.4;
        this.camera.position.copy(center).add(new THREE.Vector3(dist * 0.6, dist * 0.5, dist * 0.7));
        this.controls.target.copy(center);
        this.camera.near = Math.max(0.001, dist * 0.01);
        this.camera.far = dist * 20;
        this.camera.updateProjectionMatrix();
        this.controls.update();
    }
}
