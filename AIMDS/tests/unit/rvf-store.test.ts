/**
 * Durable storage: real @ruvector/rvf imports, no mocks of the store.
 * The only injected seam is the module loader, used to prove the
 * fallback when the optional dependency is absent.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, rmSync, existsSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { AgentDBClient } from '../../src/agentdb/client';
import { RvfStore } from '../../src/agentdb/rvf-store';
import { defaultRvfLoader } from '../../src/agentdb/store-factory';
import { Logger } from '../../src/utils/logger';
import { ThreatLevel, AgentDBConfig, ThreatIncident } from '../../src/types';

const DIM = 384;

function config(path: string): AgentDBConfig {
  return {
    path,
    embeddingDim: DIM,
    hnswConfig: { m: 16, efConstruction: 200, efSearch: 100 },
    quicSync: { enabled: false, peers: [], port: 4433 },
    memory: { maxEntries: 10000, ttl: 3600000 },
  };
}

function seeded(seed: number): number[] {
  let s = seed >>> 0;
  return Array.from({ length: DIM }, () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 0xffffffff - 0.5;
  });
}

function incident(id: string, embedding: number[], timestamp = Date.now()): ThreatIncident {
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

describe('durable rvf store', () => {
  let dir: string;
  const logger = new Logger('RvfStoreTest');

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'aidefence-rvf-'));
  });
  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  it('keeps patterns, incidents and reflexion entries across a reopen', async () => {
    const path = join(dir, 'threats.rvf');
    const a = new AgentDBClient(config(path), logger);
    await a.initialize();
    expect(a.storageKind).toBe('rvf');
    for (let i = 0; i < 5; i++) await a.storeIncident(incident(`inc-${i}`, seeded(i + 1)));
    const before = await a.getStats();
    await a.shutdown();

    expect(existsSync(path)).toBe(true);

    const b = new AgentDBClient(config(path), logger);
    await b.initialize();
    const after = await b.getStats();
    expect(after.incidents).toBe(5);
    expect(after.patterns).toBe(5);
    expect(after.memoryEntries).toBe(5);
    expect(after).toMatchObject({
      incidents: before.incidents,
      patterns: before.patterns,
      memoryEntries: before.memoryEntries,
    });

    const hits = await b.vectorSearch(seeded(3), { k: 3, threshold: 0.99 });
    expect(hits).toHaveLength(1);
    expect(hits[0].id).toBe('inc-2');
    expect(hits[0].similarity).toBeGreaterThan(0.999);
    expect(hits[0].threatLevel).toBe(ThreatLevel.HIGH);
    await b.shutdown();
  });

  it('scopes search and TTL deletes to one collection by metadata filter', async () => {
    const rvf = await defaultRvfLoader();
    const path = join(dir, 'filter.rvf');
    const store = await RvfStore.open(rvf, { path, dimension: DIM });
    const v = seeded(42);
    // Same key and same vector in two collections: ids must not collide.
    await store.insert({ collection: 'incidents', document: { id: 'x', embedding: v, timestamp: 10 } });
    await store.insert({
      collection: 'threat_patterns',
      document: { patternId: 'x', embedding: v, metadata: { description: 'p', threatLevel: 3 } },
    });
    await store.insert({ collection: 'causal_graph', document: { from: 'x', to: 'y', timestamp: 10 } });

    const hits = await store.search({ collection: 'threat_patterns', vector: v, k: 10 });
    expect(hits.map(h => h.id)).toEqual(['x']);
    expect(hits[0].metadata).toEqual({ description: 'p', threatLevel: 3 });
    expect(hits[0].embedding).toHaveLength(DIM);
    expect(hits[0].embedding[7]).toBeCloseTo(v[7], 5);

    // A vectorless document is stored and counted, but never returned by search.
    expect(await store.count({ collection: 'causal_graph' })).toBe(1);
    expect(await store.search({ collection: 'causal_graph', vector: v, k: 5 })).toEqual([]);

    // Upsert replaces in place.
    await store.upsert({
      collection: 'threat_patterns',
      document: { patternId: 'x', embedding: v, metadata: { description: 'p2', threatLevel: 4 } },
    });
    expect(await store.count({ collection: 'threat_patterns' })).toBe(1);
    const again = await store.search({ collection: 'threat_patterns', vector: v, k: 10 });
    expect(again.map(h => h.metadata.description)).toEqual(['p2']);

    // TTL cleanup on incidents leaves the other collections alone.
    await store.delete({ collection: 'incidents', filter: { timestamp: { $lt: 100 } } });
    expect(await store.count({ collection: 'incidents' })).toBe(0);
    expect(await store.count({ collection: 'causal_graph' })).toBe(1);
    expect(await store.count({ collection: 'threat_patterns' })).toBe(1);
    await store.close();

    const reopened = await RvfStore.open(rvf, { path, dimension: DIM });
    expect(await reopened.count({ collection: 'incidents' })).toBe(0);
    expect(await reopened.count({ collection: 'threat_patterns' })).toBe(1);
    expect(await reopened.count({ collection: 'causal_graph' })).toBe(1);
    await reopened.close();
  });

  it('refuses to reopen a file with a different dimension', async () => {
    const rvf = await defaultRvfLoader();
    const path = join(dir, 'dim.rvf');
    const s = await RvfStore.open(rvf, { path, dimension: DIM });
    await s.close();
    await expect(RvfStore.open(rvf, { path, dimension: 128 })).rejects.toThrow(/384-dim/);
  });

  it('falls back to the in-memory store when @ruvector/rvf cannot be loaded', async () => {
    const path = join(dir, 'never.rvf');
    const missing = async () => {
      const err = new Error("Cannot find module '@ruvector/rvf'") as Error & { code: string };
      err.code = 'MODULE_NOT_FOUND';
      throw err;
    };
    const c = new AgentDBClient(config(path), logger, { loadRvf: missing });
    await c.initialize();
    expect(c.storageKind).toBe('memory');
    await c.storeIncident(incident('m-1', seeded(9)));
    expect((await c.getStats()).incidents).toBe(1);
    await c.shutdown();
    expect(existsSync(path)).toBe(false);
  });

  it('defaults to memory and writes nothing to the working directory', async () => {
    const cwdBefore = readdirSync(process.cwd()).sort();
    for (const path of ['', ':memory:']) {
      const c = new AgentDBClient(config(path), logger);
      await c.initialize();
      expect(c.storageKind).toBe('memory');
      await c.storeIncident(incident('d-1', seeded(5)));
      await c.shutdown();
    }
    expect(readdirSync(process.cwd()).sort()).toEqual(cwdBefore);
  });
});
