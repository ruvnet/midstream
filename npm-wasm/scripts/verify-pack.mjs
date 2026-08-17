#!/usr/bin/env node
/**
 * Packaging guard for midstreamer, run automatically via `prepack`
 * (i.e. on every `npm pack` / `npm publish`).
 *
 * midstreamer@0.3.1 shipped with `main`/`module` pointing at a
 * `dist/` directory that was not in the tarball and no "." entry in
 * `exports`, making `import 'midstreamer'` impossible
 * (https://github.com/ruvnet/midstream/issues/95). This script fails
 * the pack if any entry point referenced by package.json — or any
 * build artifact those entry points load at runtime — is missing or
 * not covered by the `files` allowlist.
 *
 * Deliberately does NOT shell out to `npm pack` (prepack would
 * recurse); the checks are static.
 */

import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));

const errors = [];

/** Collect every relative path referenced by an exports map value. */
function collectExportTargets(value, out) {
  if (typeof value === 'string') {
    out.push(value);
  } else if (value && typeof value === 'object') {
    for (const nested of Object.values(value)) collectExportTargets(nested, out);
  }
  return out;
}

// 1. Entry points referenced by package.json must exist on disk.
const entryPoints = new Set();
for (const field of ['main', 'module', 'types', 'browser']) {
  if (typeof pkg[field] === 'string') entryPoints.add(pkg[field]);
}
collectExportTargets(pkg.exports ?? {}, []).forEach((p) => entryPoints.add(p));

// 2. Build artifacts the entry points load at runtime. `index.js`
// dynamically imports the wasm-pack outputs, so a pack without a
// prior build would reproduce the 0.3.1 breakage silently.
const runtimeArtifacts = [
  'pkg-node/midstream_wasm.js',
  'pkg-node/midstream_wasm_bg.wasm',
  'pkg-bundler/midstream_wasm.js',
  'pkg-bundler/midstream_wasm_bg.wasm',
  'pkg/midstream_wasm.js',
  'pkg/midstream_wasm_bg.wasm',
];

for (const rel of [...entryPoints, ...runtimeArtifacts]) {
  const clean = rel.replace(/^\.\//, '');
  if (!existsSync(join(root, clean))) {
    errors.push(`missing file: ${clean} (run \`npm run build:wasm && npm run build:bundler && npm run build:nodejs\` first?)`);
  }
}

// 3. Every entry point must be covered by the `files` allowlist so it
// actually ends up in the tarball. package.json itself is always
// included by npm.
const allowlist = pkg.files ?? [];
const covered = (rel) => {
  const clean = rel.replace(/^\.\//, '');
  if (clean === 'package.json') return true;
  return allowlist.some(
    (entry) => clean === entry || clean.startsWith(`${entry.replace(/\/$/, '')}/`)
  );
};
for (const rel of entryPoints) {
  if (!covered(rel)) {
    errors.push(`entry point ${rel} is not covered by the "files" allowlist in package.json`);
  }
}

// 4. wasm-pack writes a `.gitignore` containing `*` into each output
// directory, and npm's packlist honors nested ignore files even for
// directories on the `files` allowlist — silently emptying them from
// the tarball. This is how 0.3.1 shipped without its wasm artifacts.
// The build:* scripts strip these; fail hard if one survived.
for (const dir of ['pkg', 'pkg-node', 'pkg-bundler', 'types']) {
  for (const ignoreFile of ['.gitignore', '.npmignore']) {
    if (existsSync(join(root, dir, ignoreFile))) {
      errors.push(
        `${dir}/${ignoreFile} exists — npm would silently exclude ${dir}/ from the tarball. ` +
        `Delete it (the build:* scripts do this automatically).`
      );
    }
  }
}

// 5. The "." export must exist — the exact regression from issue #95.
if (!pkg.exports || !pkg.exports['.']) {
  errors.push('package.json "exports" has no "." entry — `import \'midstreamer\'` would throw ERR_PACKAGE_PATH_NOT_EXPORTED');
}

if (errors.length > 0) {
  console.error('verify-pack: refusing to pack midstreamer:');
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}

console.log('verify-pack: all entry points present and included in the tarball.');
