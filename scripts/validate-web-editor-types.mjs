import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');

const editorTypesPath = path.join(repoRoot, 'web/src/procgeo-editor-types.d.ts');
const wasmTypesCandidates = [
  path.join(repoRoot, 'web/wasm/procgeo_wasm.d.ts'),
  path.join(repoRoot, 'bindings/procgeo-wasm/pkg/procgeo_wasm.d.ts'),
  path.join(repoRoot, 'web/pkg/procgeo_wasm.d.ts'),
];

const wasmTypesPath = wasmTypesCandidates.find((candidate) => fs.existsSync(candidate));

if (!wasmTypesPath) {
  console.error(
    'validate-web-editor-types: no generated procgeo_wasm.d.ts found. ' +
      'Expected one of:\n' +
      wasmTypesCandidates.map((candidate) => `  - ${path.relative(repoRoot, candidate)}`).join('\n')
  );
  process.exit(1);
}

const editorSource = fs.readFileSync(editorTypesPath, 'utf8');
const wasmSource = fs.readFileSync(wasmTypesPath, 'utf8');

function extractBlock(source, header) {
  const start = source.indexOf(header);
  if (start === -1) {
    throw new Error(`Missing block header: ${header}`);
  }

  const braceStart = source.indexOf('{', start);
  if (braceStart === -1) {
    throw new Error(`Missing opening brace for: ${header}`);
  }

  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(braceStart + 1, index);
      }
    }
  }

  throw new Error(`Unterminated block for: ${header}`);
}

function extractMembers(blockSource) {
  const members = new Set();
  const memberPattern = /^\s*(?:readonly\s+)?([A-Za-z_]\w*)(?:\??:\s|\()/gm;

  for (const match of blockSource.matchAll(memberPattern)) {
    members.add(match[1]);
  }

  return members;
}

function extractWasmModuleMembers(source) {
  const members = new Set();
  const classPattern = /^export class ([A-Za-z_]\w+)/gm;
  const functionPattern = /^export function ([A-Za-z_]\w+)\(/gm;
  const ignoredFunctions = new Set(['initSync']);

  for (const match of source.matchAll(classPattern)) {
    members.add(match[1]);
  }

  for (const match of source.matchAll(functionPattern)) {
    const name = match[1];
    if (!ignoredFunctions.has(name)) {
      members.add(name);
    }
  }

  return members;
}

function extractWasmClassMembers(source, className) {
  const block = extractBlock(source, `export class ${className}`);
  const members = new Set();
  const memberPattern = /^\s*(?:private\s+)?(?:readonly\s+)?([A-Za-z_]\w*)(?:\(|:)/gm;
  const ignoredMembers = new Set(['constructor']);

  for (const match of block.matchAll(memberPattern)) {
    const name = match[1];
    if (!ignoredMembers.has(name)) {
      members.add(name);
    }
  }

  return members;
}

function missingMembers(expected, actual) {
  return [...expected].filter((name) => !actual.has(name)).sort();
}

const wasmModuleMembers = extractWasmModuleMembers(wasmSource);
const editorModuleMembers = extractMembers(extractBlock(editorSource, 'interface ProcGeoModule'));

const wasmGeometryMembers = extractWasmClassMembers(wasmSource, 'Geometry');
const editorGeometryMembers = extractMembers(extractBlock(editorSource, 'interface ProcGeoGeometry'));

const wasmCopImageMembers = extractWasmClassMembers(wasmSource, 'CopImage');
const editorCopImageMembers = extractMembers(extractBlock(editorSource, 'interface ProcGeoCopImage'));

const missingModule = missingMembers(wasmModuleMembers, editorModuleMembers);
const missingGeometry = missingMembers(wasmGeometryMembers, editorGeometryMembers);
const missingCopImage = missingMembers(wasmCopImageMembers, editorCopImageMembers);

if (missingModule.length || missingGeometry.length || missingCopImage.length) {
  console.error('Web editor typings are out of sync with the generated WASM bindings.');

  if (missingModule.length) {
    console.error(`Missing ProcGeoModule members: ${missingModule.join(', ')}`);
  }

  if (missingGeometry.length) {
    console.error(`Missing ProcGeoGeometry members: ${missingGeometry.join(', ')}`);
  }

  if (missingCopImage.length) {
    console.error(`Missing ProcGeoCopImage members: ${missingCopImage.join(', ')}`);
  }

  console.error(
    `Update ${path.relative(repoRoot, editorTypesPath)} to cover ${path.relative(repoRoot, wasmTypesPath)}.`
  );
  process.exit(1);
}

console.log(
  `Web editor typings validated against ${path.relative(repoRoot, wasmTypesPath)}.`
);
