#!/usr/bin/env node
// Threat-pattern lookup latency: InMemoryStore (linear scan) vs RvfStore (rvf HNSW).
// Usage: npm run build && node scripts/bench-store.mjs [sizes=1000,10000] [queries=200]
// Prints one JSON line per (backend, size). Writes only to a fresh OS temp dir.
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { InMemoryStore, RvfStore } = require('../dist/index.js');
const { defaultRvfLoader } = require('../dist/agentdb/store-factory.js');

const DIM = 384;
const K = 10;
const sizes = (process.argv[2] ?? '1000,10000').split(',').map(Number);
const nQueries = Number(process.argv[3] ?? 200);

function rng(seed) {
  let s = seed >>> 0;
  return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 0xffffffff) - 0.5;
}
function vec(r) {
  return Array.from({ length: DIM }, r);
}
function pct(sorted, p) {
  return sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))];
}

async function run(kind, store, n) {
  const r = rng(n);
  const patterns = [];
  const t0 = performance.now();
  for (let i = 0; i < n; i++) {
    const v = vec(r);
    patterns.push(v);
    await store.insert({
      collection: 'threat_patterns',
      document: { patternId: `p${i}`, embedding: v, metadata: { threatLevel: 2 } },
    });
  }
  const insertMs = performance.now() - t0;

  // Queries are stored patterns plus noise, so each has a true nearest neighbour.
  const qr = rng(n + 7);
  const queries = Array.from({ length: nQueries }, (_, i) =>
    patterns[(i * 37) % n].map(x => x + 0.05 * qr())
  );
  for (const q of queries.slice(0, 20)) await store.search({ collection: 'threat_patterns', vector: q, k: K });
  const lat = [];
  const results = [];
  for (const q of queries) {
    const s = performance.now();
    results.push(await store.search({ collection: 'threat_patterns', vector: q, k: K, ef: 100 }));
    lat.push(performance.now() - s);
  }
  lat.sort((a, b) => a - b);
  return {
    kind, n, dim: DIM, k: K, queries: nQueries,
    p50_ms: +pct(lat, 50).toFixed(4),
    p95_ms: +pct(lat, 95).toFixed(4),
    insert_ms_per_doc: +(insertMs / n).toFixed(4),
    results,
  };
}

const rvf = await defaultRvfLoader();
for (const n of sizes) {
  const mem = await run('memory', new InMemoryStore(), n);
  const dir = mkdtempSync(join(tmpdir(), 'aidefence-bench-'));
  try {
    const path = join(dir, 'bench.rvf');
    const store = await RvfStore.open(rvf, { path, dimension: DIM, m: 16, efConstruction: 200 });
    const disk = await run('rvf', store, n);
    await store.close();
    const o = performance.now();
    const re = await RvfStore.open(rvf, { path, dimension: DIM });
    const reopenMs = performance.now() - o;
    await re.close();
    // recall@K of the HNSW results against the exact linear scan
    let hit = 0;
    for (let i = 0; i < mem.results.length; i++) {
      const truth = new Set(mem.results[i].map(x => x.id));
      hit += disk.results[i].filter(x => truth.has(x.id)).length;
    }
    const recall = +(hit / (mem.results.length * K)).toFixed(4);
    delete mem.results;
    delete disk.results;
    console.log(JSON.stringify(mem));
    console.log(JSON.stringify({ ...disk, recall_at_k: recall, reopen_ms: +reopenMs.toFixed(1) }));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}
