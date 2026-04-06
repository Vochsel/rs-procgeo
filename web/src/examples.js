export const examples = {
    basic: `// Basic box with normals
const box = pg.createBox({ size: [1, 1, 1] });
const withNormals = pg.computeNormals(box);
return withNormals;
`,

    subdiv: `// Catmull-Clark subdivided sphere
let geo = pg.createSphere({ radius: 0.8, rows: 4, cols: 8 });
geo = pg.subdivide(geo, { depth: 2, mode: 'catmullClark' });
geo = pg.computeNormals(geo);
return geo;
`,

    scatter: `// Scatter boxes on a grid
const grid = pg.createGrid({ rows: 5, cols: 5, sizeX: 4, sizeY: 4 });
const points = pg.scatter(grid, { count: 30, seed: 42 });
const box = pg.createBox({ size: [0.15, 0.15, 0.15] });
let instances = pg.copyToPoints(box, points);
instances = pg.computeNormals(instances);
instances = pg.color(instances, { color: [0.9, 0.5, 0.2] });
return instances;
`,

    extrude: `// Extruded city blocks
const grid = pg.createGrid({ rows: 2, cols: 2, sizeX: 8, sizeY: 8 });
const points = pg.scatter(grid, { count: 12, seed: 7 });
let building = pg.createBox({ size: [0.8, 2, 0.8] });
building = pg.polyExtrude(building, { distance: 0.3, inset: 0.15 });
let city = pg.copyToPoints(building, points);
city = pg.computeNormals(city);
city = pg.color(city, { color: [0.6, 0.65, 0.75] });

const ground = pg.createGrid({ rows: 2, cols: 2, sizeX: 10, sizeY: 10 });
const groundN = pg.computeNormals(ground);
const groundC = pg.color(groundN, { color: [0.25, 0.3, 0.2] });

return pg.fuse(city, { distance: 0.001 });
`,

    fracture: `// Voronoi fractured box
let box = pg.createBox({ size: [1.5, 1.5, 1.5] });
let points = pg.scatter(box, { count: 6, seed: 42 });
let fractured = pg.voronoiFracture(box, points);
fractured = pg.computeNormals(fractured);
return fractured;
`,
};
