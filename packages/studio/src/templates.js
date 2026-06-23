// Starter templates, expressed in the round-trip node DSL. These mirror the web
// playground examples that map onto the current node catalog.

export const TEMPLATES = [
  {
    name: 'Basic Box',
    description: 'A single box with normals.',
    code: `const box1 = box({ size: [2, 2, 2] })
const normal1 = normal(box1)
return normal1
`,
  },
  {
    name: 'Subdivided Box',
    description: 'Catmull-style subdivision on a box.',
    code: `const box1 = box({ size: [2, 2, 2] })
const subdivide1 = subdivide(box1, { depth: 2, mode: 'CatmullClark' })
const normal1 = normal(subdivide1)
return normal1
`,
  },
  {
    name: 'Smooth Sphere',
    description: 'A sphere relaxed with the smooth SOP.',
    code: `const sphere1 = sphere({ rows: 24, cols: 48 })
const smooth1 = smooth(sphere1, { iterations: 5, strength: 0.6 })
const normal1 = normal(smooth1)
return normal1
`,
  },
  {
    name: 'Extruded Grid',
    description: 'A grid pushed out with polyextrude.',
    code: `const grid1 = grid({ size: [6, 6], rows: 8, cols: 8 })
const polyextrude1 = polyextrude(grid1, { distance: 0.5, inset: 0.08 })
const normal1 = normal(polyextrude1)
return normal1
`,
  },
  {
    name: 'Scatter on Grid',
    description: 'Points scattered across a grid surface.',
    code: `const grid1 = grid({ size: [8, 8], rows: 12, cols: 12 })
const scatter1 = scatter(grid1, { count: 400, seed: 1 })
return scatter1
`,
  },
  {
    name: 'Copy to Points',
    description: 'Copy a small box onto scattered points.',
    code: `const grid1 = grid({ size: [8, 8], rows: 10, cols: 10 })
const scatter1 = scatter(grid1, { count: 120, seed: 3 })
const box1 = box({ size: [0.3, 0.6, 0.3] })
const copy_to_points1 = copy_to_points(box1, scatter1)
const normal1 = normal(copy_to_points1)
return normal1
`,
  },
  {
    name: 'Merged Primitives',
    description: 'Box, sphere and torus merged into one.',
    code: `const box1 = box({ size: [1.5, 1.5, 1.5] })
const sphere1 = sphere()
const sphere2 = transform(sphere1, { translate: [2.5, 0, 0] })
const torus1 = torus({ center: [-2.5, 0, 0] })
const merge1 = merge(box1, sphere2, torus1)
const normal1 = normal(merge1)
return normal1
`,
  },
  {
    name: 'Subdivided Torus',
    description: 'A smooth, dense torus.',
    code: `const torus1 = torus({ rows: 20, cols: 36 })
const subdivide1 = subdivide(torus1, { depth: 1, mode: 'CatmullClark' })
const normal1 = normal(subdivide1)
return normal1
`,
  },
];
