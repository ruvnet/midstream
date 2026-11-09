/**
 * Write batching on the durable rvf store: real @ruvector/rvf, real files.
 * The only seam is a counting wrapper around the real rvf database, so
 * the tests can see how many commits a sequence of writes produced.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { AgentDBClient } from '../../src/agentdb/client';
import { RvfStore, RvfModule, RvfDatabaseLike } from '../../src/agentdb/rvf-store';
import { defaultRvfLoader } from '../../src/agentdb/store-factory';
import { Logger } from '../../src/utils/logger';
import { ThreatLevel, AgentDBConfig, ThreatIncident } from '../../src/types';

const DIM = 16;

function seeded(seed: number, dim = DIM): number[] {
  let s = seed >>> 0;
  return Array.from({ length: dim }, () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 0xffffffff - 0.5;
  });
}

interface Calls {
  ingest: number[];
  delete: number[];
}

/** The real rvf module, with the database's write calls counted. */
async function countingRvf(): Promise<{ rvf: RvfModule; calls: Calls }> {
  const real = await defaultRvfLoader();
  const calls: Calls = { ingest: [], delete: [] };
  const wrap = (db: RvfDatabaseLike): RvfDatabaseLike => ({
    ingestBatch: entries => {
      calls.ingest.push(entries.length);
      return db.ingestBatch(entries);
    },
    delete: ids => {
      calls.delete.push(ids.length);
      return db.delete(ids);
    },
    query: (v, k, o) => db.query(v, k, o),
    status: () => db.status(),
    dimension: () => db.dimension(),
    close: () => db.close(),
  });
  const rvf: RvfModule = {
    RvfDatabase: {
      create: async (p, o) => wrap(await real.RvfDatabase.create(p, o)),
      open: async p => wrap(await real.RvfDatabase.open(p)),
    },
  };
  return { rvf, calls };
}

/** Every vector in the file, read with the raw rvf API. */
async function rawDocIds(path: string): Promise<string[]> {
  const rvf = await defaultRvfLoader();
  const db = await rvf.RvfDatabase.open(path);
  const { totalVectors } = await db.status();
  const placeholder = new Array(await db.dimension()).fill(0);
  placeholder[0] = 1;
  const hits = totalVectors ? await db.query(placeholder, totalVectors, { forceExact: true }) : [];
  await db.close();
  return hits.map(h => `${h.metadata?.collection}/${h.metadata?.docId}`);
}

function incident(id: string, embedding: number[]): ThreatIncident {
  const timestamp = Date.now();
  return {
    id,
    timestamp,
    request: {
      id: `req-${id}`,
      timestamp,
      source: { ip: '10.0.0.1', headers: {} },
      action: { type: 'read', resource: '/api/data', method: 'GET' },
    },
    result: {
      allowed: false,
      confidence: 0.95,
      latencyMs: 3,
      threatLevel: ThreatLevel.HIGH,
      matches: [],
      metadata: { vectorSearchTime: 1, verificationTime: 2, totalTime: 3, pathTaken: 'deep' },
    },
    embedding,
  };
}

/**
 * Run the crash-writer child and SIGKILL it once it reports ready.
 * rvf only breaks a dead writer's lock once it is 30 s old, so the test
 * removes the killed child's `<path>.lock` itself, as a supervisor
 * restarting the process sooner would have to.
 */
function crashWriter(args: (string | number)[]): Promise<void> {
  const fixture = join(__dirname, '..', 'fixtures', 'rvf-crash-writer.ts');
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, ['--import', 'tsx', fixture, ...args.map(String)], {
      stdio: ['ignore', 'pipe', 'pipe'],
      env: process.env,
    });
    let err = '';
    const guard = setTimeout(() => {
      child.kill('SIGKILL');
      reject(new Error(`crash writer timed out: ${err}`));
    }, 25000);
    child.stderr.on('data', d => (err += d));
    child.stdout.on('data', d => {
      if (String(d).includes('ready')) child.kill('SIGKILL');
    });
    child.on('exit', (code, signal) => {
      clearTimeout(guard);
      if (signal === 'SIGKILL') {
        const lock = `${args[0]}.lock`;
        if (existsSync(lock)) rmSync(lock);
        resolve();
      }
      else reject(new Error(`crash writer exited ${code}: ${err}`));
    });
  });
}

describe('rvf write batching', () => {
  let dir: string;

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'aidefence-batch-'));
  });
  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  it('coalesces many writes into a few commits', async () => {
    const { rvf, calls } = await countingRvf();
    const path = join(dir, 'coalesce.rvf');
    const store = await RvfStore.open(rvf, { path, dimension: DIM, maxBatch: 256, flushIntervalMs: 10000 });
    for (let i = 0; i < 600; i++) {
      await store.insert({ collection: 'incidents', document: { id: `i${i}`, embedding: seeded(i), timestamp: i } });
    }
    await store.flush();
    expect(calls.ingest.reduce((a, b) => a + b, 0)).toBe(600);
    expect(calls.ingest.length).toBeLessThanOrEqual(4);
    expect(Math.max(...calls.ingest)).toBeLessThanOrEqual(512);
    expect(calls.delete).toEqual([]);
    await store.close();
  });

  it('commits on the timer without an explicit flush', async () => {
    const { rvf, calls } = await countingRvf();
    const store = await RvfStore.open(rvf, { path: join(dir, 't.rvf'), dimension: DIM, flushIntervalMs: 20 });
    for (let i = 0; i < 5; i++) await store.insert({ collection: 'c', document: { id: `k${i}`, embedding: seeded(i) } });
    expect(calls.ingest).toEqual([]);
    await new Promise(r => setTimeout(r, 120));
    expect(calls.ingest).toEqual([5]);
    await store.close();
  });

  it('flushIntervalMs 0 commits every write before it resolves', async () => {
    const { rvf, calls } = await countingRvf();
    const store = await RvfStore.open(rvf, { path: join(dir, 'wt.rvf'), dimension: DIM, flushIntervalMs: 0 });
    await store.insert({ collection: 'c', document: { id: 'a', embedding: seeded(1) } });
    expect(calls.ingest).toEqual([1]);
    await store.insert({ collection: 'c', document: { id: 'b', embedding: seeded(2) } });
    expect(calls.ingest).toEqual([1, 1]);
    await store.close();
  });

  it('stores an incident with one commit instead of three', async () => {
    const { rvf, calls } = await countingRvf();
    const path = join(dir, 'incident.rvf');
    const cfg: AgentDBConfig = {
      path,
      embeddingDim: DIM,
      hnswConfig: { m: 16, efConstruction: 200, efSearch: 100 },
      quicSync: { enabled: false, peers: [], port: 4433 },
      memory: { maxEntries: 10000, ttl: 3600000 },
      writeBehind: { flushIntervalMs: 10000 },
    };
    const client = new AgentDBClient(cfg, new Logger('BatchTest'), { loadRvf: async () => rvf });
    await client.initialize();
    await client.storeIncident(incident('one', seeded(7)));
    await client.flush();
    // incident + threat pattern + reflexion entry, in a single ingest
    expect(calls.ingest).toEqual([3]);

    for (let i = 0; i < 10; i++) await client.storeIncident(incident(`n${i}`, seeded(100 + i)));
    await client.flush();
    expect(calls.ingest).toEqual([3, 30]);
    expect(await client.getStats()).toMatchObject({ incidents: 11, patterns: 11, memoryEntries: 11 });
    await client.shutdown();
  });

  it('shows queued documents to search and count before they are committed', async () => {
    const { rvf, calls } = await countingRvf();
    const store = await RvfStore.open(rvf, { path: join(dir, 'ryw.rvf'), dimension: DIM, flushIntervalMs: 10000 });
    const v = seeded(11);
    await store.insert({ collection: 'threat_patterns', document: { patternId: 'p', embedding: v, metadata: { rev: 1 } } });
    await store.flush();
    await store.upsert({ collection: 'threat_patterns', document: { patternId: 'p', embedding: v, metadata: { rev: 2 } } });
    await store.insert({ collection: 'threat_patterns', document: { patternId: 'q', embedding: seeded(12), metadata: { rev: 1 } } });
    expect(calls.ingest).toEqual([1]);

    expect(await store.count({ collection: 'threat_patterns' })).toBe(2);
    const hits = await store.search({ collection: 'threat_patterns', vector: v, k: 10 });
    expect(hits.map(h => [h.id, h.metadata.rev])).toEqual([
      ['p', 2],
      ['q', 1],
    ]);
    expect(hits[0].similarity).toBeGreaterThan(0.999);
    await store.close();
  });

  it('flush() persists, and a reopen sees everything that was flushed', async () => {
    const path = join(dir, 'reopen.rvf');
    const rvf = await defaultRvfLoader();
    const a = await RvfStore.open(rvf, { path, dimension: DIM, flushIntervalMs: 10000 });
    for (let i = 0; i < 300; i++) {
      await a.insert({ collection: 'threat_patterns', document: { patternId: `p${i}`, embedding: seeded(i), metadata: { i } } });
    }
    await a.flush();
    await a.close();

    const ids = await rawDocIds(path);
    expect(ids).toHaveLength(300);

    const b = await RvfStore.open(rvf, { path, dimension: DIM });
    expect(await b.count({ collection: 'threat_patterns' })).toBe(300);
    const hit = await b.search({ collection: 'threat_patterns', vector: seeded(123), k: 1, ef: 200 });
    expect(hit[0]).toMatchObject({ id: 'p123', metadata: { i: 123 } });
    await b.close();
  });

  it('close() commits what is still queued', async () => {
    const path = join(dir, 'close.rvf');
    const rvf = await defaultRvfLoader();
    const a = await RvfStore.open(rvf, { path, dimension: DIM, flushIntervalMs: 10000 });
    for (let i = 0; i < 20; i++) await a.insert({ collection: 'c', document: { id: `k${i}`, embedding: seeded(i) } });
    await a.close();
    expect(await rawDocIds(path)).toHaveLength(20);
    await expect(a.insert({ collection: 'c', document: { id: 'late' } })).rejects.toThrow(/closed/);
  });

  it('a crash without flush loses at most the unflushed writes', async () => {
    // Long interval: the 30 writes after the flush are still queued at the kill.
    const lost = join(dir, 'crash-queued.rvf');
    await crashWriter([lost, 50, 30, 60000, 0]);
    const afterKill = (await rawDocIds(lost)).sort();
    expect(afterKill.length).toBeGreaterThanOrEqual(50);
    expect(afterKill.length).toBeLessThanOrEqual(80);
    for (let i = 0; i < 50; i++) expect(afterKill).toContain(`incidents/d${i}`);

    // Short interval, killed well after it elapsed: nothing is lost.
    const kept = join(dir, 'crash-timer.rvf');
    await crashWriter([kept, 50, 30, 50, 500]);
    expect(await rawDocIds(kept)).toHaveLength(80);

    // The killed store reopens cleanly and accepts writes.
    const rvf = await defaultRvfLoader();
    const s = await RvfStore.open(rvf, { path: lost, dimension: 8 });
    expect(await s.count({ collection: 'incidents' })).toBe(afterKill.length);
    await s.insert({ collection: 'incidents', document: { id: 'after', timestamp: 1 } });
    await s.close();
    expect(await rawDocIds(lost)).toHaveLength(afterKill.length + 1);
  });

  it('concurrent writers do not interleave badly: last write wins, no duplicates', async () => {
    const path = join(dir, 'concurrent.rvf');
    const rvf = await defaultRvfLoader();
    const store = await RvfStore.open(rvf, { path, dimension: DIM, maxBatch: 32, flushIntervalMs: 5 });
    const KEYS = 40;
    const expected = new Map<string, string>();
    const writer = async (w: number) => {
      for (let i = 0; i < 60; i++) {
        const key = `k${(w * 7 + i) % KEYS}`;
        const tag = `w${w}-${i}`;
        // The queue records writes in call order, so this is the order that must win.
        expected.set(key, tag);
        const p = store.upsert({
          collection: 'threat_patterns',
          document: { patternId: key, embedding: seeded(Number(key.slice(1)) + 1), metadata: { tag } },
        });
        if (i % 9 === 0) await Promise.all([p, store.flush()]);
        else if (i % 5 === 0) await Promise.all([p, store.search({ collection: 'threat_patterns', vector: seeded(i), k: 5 })]);
        else await p;
      }
    };
    await Promise.all(Array.from({ length: 8 }, (_, w) => writer(w)));
    await store.flush();
    expect(await store.count({ collection: 'threat_patterns' })).toBe(KEYS);
    const check = async (s: RvfStore) => {
      for (const [key, tag] of expected) {
        const [hit] = await s.search({ collection: 'threat_patterns', vector: seeded(Number(key.slice(1)) + 1), k: 1, ef: 200 });
        expect([hit.id, hit.metadata.tag]).toEqual([key, tag]);
      }
    };
    await check(store);
    await store.close();

    const ids = await rawDocIds(path);
    expect(ids).toHaveLength(KEYS);
    expect(new Set(ids).size).toBe(KEYS);
    const reopened = await RvfStore.open(rvf, { path, dimension: DIM });
    await check(reopened);
    await reopened.close();
  });

  it('upsert stays duplicate-free across flushes and reopens, and keeps the HNSW index', async () => {
    const { rvf, calls } = await countingRvf();
    const path = join(dir, 'upsert.rvf');
    const n = 2000;
    const a = await RvfStore.open(rvf, { path, dimension: DIM, flushIntervalMs: 10000 });
    await a.insertMany(
      Array.from({ length: n }, (_, i) => ({
        collection: 'threat_patterns',
        document: { patternId: `p${i}`, embedding: seeded(i), metadata: { rev: 0 } },
      }))
    );
    await a.flush();
    await a.search({ collection: 'threat_patterns', vector: seeded(1), k: 10 }); // builds the index

    for (let rev = 1; rev <= 3; rev++) {
      await a.upsert({ collection: 'threat_patterns', document: { patternId: 'p5', embedding: seeded(5), metadata: { rev } } });
      await a.upsert({ collection: 'threat_patterns', document: { patternId: 'p5', embedding: seeded(5), metadata: { rev } } });
      await a.flush();
      // An id re-ingested while live makes rvf drop and rebuild its index
      // (seconds at this size); the delete-first commit avoids that.
      const t = performance.now();
      const [hit] = await a.search({ collection: 'threat_patterns', vector: seeded(5), k: 1 });
      expect(performance.now() - t).toBeLessThan(250);
      expect(hit).toMatchObject({ id: 'p5', metadata: { rev } });
    }
    // Each round: one delete of the replaced id, one ingest of the new version.
    expect(calls.delete).toEqual([1, 1, 1]);
    expect(calls.ingest.slice(-3)).toEqual([1, 1, 1]);
    expect(await a.count({ collection: 'threat_patterns' })).toBe(n);
    await a.close();

    // A key hydrated from an earlier session is replaced, not duplicated.
    const b = await RvfStore.open(rvf, { path, dimension: DIM });
    await b.upsert({ collection: 'threat_patterns', document: { patternId: 'p5', embedding: seeded(5), metadata: { rev: 9 } } });
    await b.close();
    const ids = await rawDocIds(path);
    expect(ids).toHaveLength(n);
    expect(ids.filter(id => id === 'threat_patterns/p5')).toHaveLength(1);

    const c = await RvfStore.open(rvf, { path, dimension: DIM });
    const [hit] = await c.search({ collection: 'threat_patterns', vector: seeded(5), k: 1 });
    expect(hit).toMatchObject({ id: 'p5', metadata: { rev: 9 } });
    await c.close();
  });

  it('TTL delete commits the queue first and applies to queued documents', async () => {
    const store = await RvfStore.open(await defaultRvfLoader(), {
      path: join(dir, 'ttl.rvf'),
      dimension: DIM,
      flushIntervalMs: 10000,
    });
    await store.insert({ collection: 'incidents', document: { id: 'old', timestamp: 1 } });
    await store.insert({ collection: 'incidents', document: { id: 'new', timestamp: 1000 } });
    await store.delete({ collection: 'incidents', filter: { timestamp: { $lt: 500 } } });
    expect(await store.count({ collection: 'incidents' })).toBe(1);
    await store.close();
  });
});
