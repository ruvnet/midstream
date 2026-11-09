/**
 * Durable storage backend for AgentDBClient, on @ruvector/rvf.
 *
 * All collections share one `.rvf` file. Each document is one vector
 * whose per-vector metadata carries the collection name, the document
 * key and the document itself (JSON), so a reopen restores everything
 * without a separate sidecar. Search is rvf's HNSW index, scoped to a
 * collection with a name-based metadata filter.
 *
 * Writes go through a write-behind queue (./write-behind.ts): documents
 * are coalesced into one rvf commit (one `delete` for replaced keys, one
 * `ingestBatch`) per batch, instead of one or two commits per document.
 * `insert` resolves once a document is queued; `await flush()` makes
 * everything written before it durable. Queued documents are visible to
 * `search` and `count` before they are committed.
 *
 * @ruvector/rvf is an optional dependency. It is loaded lazily by
 * `createStore` (./store-factory.ts) only when a storage path is set;
 * this module imports nothing from it at load time.
 */

import { randomUUID } from 'node:crypto';
import { existsSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import {
  Doc,
  SearchInput,
  InsertInput,
  UpsertInput,
  DeleteInput,
  CountInput,
  CollectionInput,
  IndexInput,
  SyncInput,
  SearchResult,
  StoreBackend,
  docKey,
} from './store-types';
import { WriteBehind, WriteBehindOptions } from './write-behind';

/** The parts of the @ruvector/rvf surface this backend uses. */
type FilterExpr =
  | { op: 'eq' | 'lt'; field: string; value: string | number | boolean }
  | { op: 'and'; exprs: FilterExpr[] };

interface RvfHit {
  id: string;
  distance: number;
  metadata?: Record<string, unknown>;
}

interface RvfEntry {
  id: string;
  vector: number[] | Float32Array;
  metadata?: Record<string, unknown>;
}

export interface RvfDatabaseLike {
  ingestBatch(entries: RvfEntry[]): Promise<{ accepted: number; rejected: number }>;
  delete(ids: string[]): Promise<{ deleted: number }>;
  query(
    vector: number[] | Float32Array,
    k: number,
    options?: { efSearch?: number; filter?: FilterExpr; forceExact?: boolean }
  ): Promise<RvfHit[]>;
  status(): Promise<{ totalVectors: number; fileSizeBytes: number }>;
  dimension(): Promise<number>;
  close(): Promise<void>;
}

export interface RvfModule {
  RvfDatabase: {
    create(
      path: string,
      options: { dimensions: number; metric?: 'cosine'; m?: number; efConstruction?: number }
    ): Promise<RvfDatabaseLike>;
    open(path: string): Promise<RvfDatabaseLike>;
  };
}

export interface RvfStoreOptions extends WriteBehindOptions {
  path: string;
  dimension: number;
  m?: number;
  efConstruction?: number;
  /** Called when a commit nobody awaited fails; the next flush() also throws it. */
  onWriteError?: (error: Error) => void;
}

/** One queued document. */
interface Pending {
  collection: string;
  key: string;
  entry: RvfEntry;
  /** The document's own embedding, if it has a usable one. */
  vec?: number[];
  metadata: Record<string, unknown>;
}

export class RvfStore implements StoreBackend {
  readonly kind = 'rvf' as const;
  /** collection -> document keys stored or queued */
  private index = new Map<string, Set<string>>();
  /** collection -> document keys committed to the file */
  private onDisk = new Map<string, Set<string>>();
  private fileSizeBytes = 0;
  /** Stands in for the vector of a document that has none of the right size. */
  private readonly placeholder: number[];
  private readonly queue: WriteBehind<string, Pending>;

  private constructor(
    private db: RvfDatabaseLike,
    private dim: number,
    opts: WriteBehindOptions & { onWriteError?: (error: Error) => void }
  ) {
    this.placeholder = new Array(dim).fill(0);
    this.placeholder[0] = 1;
    this.queue = new WriteBehind(batch => this.commit(batch), opts, opts.onWriteError);
  }

  static async open(rvf: RvfModule, opts: RvfStoreOptions): Promise<RvfStore> {
    let db: RvfDatabaseLike;
    if (existsSync(opts.path)) {
      db = await rvf.RvfDatabase.open(opts.path);
      const dim = await db.dimension();
      if (dim !== opts.dimension) {
        await db.close();
        throw new Error(
          `aidefence: ${opts.path} holds ${dim}-dim vectors but embeddingDim is ${opts.dimension}`
        );
      }
    } else {
      mkdirSync(dirname(opts.path), { recursive: true });
      db = await rvf.RvfDatabase.create(opts.path, {
        dimensions: opts.dimension,
        metric: 'cosine',
        m: opts.m,
        efConstruction: opts.efConstruction,
      });
    }
    const store = new RvfStore(db, opts.dimension, opts);
    await store.hydrate();
    return store;
  }

  /** Writes per commit, and the longest a queued write waits (0 = write-through). */
  get writeBehind(): { maxBatch: number; flushIntervalMs: number } {
    return { maxBatch: this.queue.maxBatch, flushIntervalMs: this.queue.flushIntervalMs };
  }

  async createIndex(_: IndexInput): Promise<void> {
    // The HNSW index is built by rvf as vectors are ingested.
  }

  async createCollection(input: CollectionInput): Promise<void> {
    this.keys(this.index, input.name);
  }

  async insert(input: InsertInput): Promise<void> {
    return this.insertMany([input]);
  }

  /** Queue several documents at once; they share one commit. */
  async insertMany(inputs: InsertInput[]): Promise<void> {
    if (this.queue.closed) throw new Error('aidefence: store is closed');
    const queued: [string, Pending][] = [];
    for (const input of inputs) {
      const p = this.prepare(input);
      queued.push([rvfId(p.collection, p.key), p]);
    }
    for (const [, p] of queued) this.keys(this.index, p.collection).add(p.key);
    await this.queue.put(queued);
  }

  async upsert(input: UpsertInput): Promise<void> {
    // A second write to the same key replaces the first, as in InMemoryStore.
    return this.insert(input);
  }

  async flush(): Promise<void> {
    await this.queue.flush();
  }

  async search(input: SearchInput): Promise<SearchResult[]> {
    const keys = this.index.get(input.collection);
    if (!keys || keys.size === 0 || input.vector.length !== this.dim) return [];

    // Queued documents of this collection; they shadow committed versions.
    const queued: Pending[] = [];
    let shadowed = 0;
    const committed = this.onDisk.get(input.collection);
    for (const p of this.queue.uncommitted()) {
      if (p.collection !== input.collection) continue;
      queued.push(p);
      if (committed?.has(p.key)) shadowed++;
    }

    const out: SearchResult[] = [];
    if (committed && committed.size > 0) {
      const hits = await this.db.query(
        input.vector,
        Math.min(input.k + shadowed, committed.size),
        {
          efSearch: input.ef,
          filter: {
            op: 'and',
            exprs: [
              { op: 'eq', field: 'collection', value: input.collection },
              { op: 'eq', field: 'hasVector', value: 1 },
            ],
          },
        }
      );
      for (const hit of hits) {
        const m = hit.metadata;
        if (!m || typeof m.docId !== 'string' || !keys.has(m.docId)) continue;
        if (this.queue.get(hit.id)) continue; // a newer version is queued
        const doc = JSON.parse(String(m.doc)) as Doc;
        out.push({
          id: m.docId,
          similarity: 1 - hit.distance,
          embedding: typeof m.vec === 'string' ? decodeVector(m.vec) : [],
          metadata: (doc.metadata as Record<string, unknown>) ?? {},
        });
      }
    }
    // Exact scan of the queue: at most a couple of batches.
    for (const p of queued) {
      if (!p.vec) continue;
      out.push({
        id: p.key,
        similarity: cosine(input.vector, p.vec),
        embedding: p.vec,
        metadata: p.metadata,
      });
    }
    out.sort((a, b) => b.similarity - a.similarity);
    return out.slice(0, input.k);
  }

  async count(input: CountInput): Promise<number> {
    this.fileSizeBytes = (await this.db.status()).fileSizeBytes;
    return this.index.get(input.collection)?.size ?? 0;
  }

  /** On-disk size of the .rvf file, as of the last `count()`. */
  getMemoryUsage(): number {
    return this.fileSizeBytes;
  }

  async delete(input: DeleteInput): Promise<void> {
    const cutoff = input.filter.timestamp?.$lt;
    if (cutoff === undefined) return;
    // Commit the queue first, so a queued document is judged by the same rule.
    await this.queue.flush();
    await this.queue.exclusive(() => this.deleteOlderThan(input.collection, cutoff));
  }

  async sync(_: SyncInput): Promise<void> {
    // Peer sync is not implemented for either backend.
  }

  /** Commit everything queued, then close the file. */
  async close(): Promise<void> {
    try {
      await this.queue.close();
    } finally {
      await this.db.close();
      this.index.clear();
      this.onDisk.clear();
    }
  }

  private prepare(input: InsertInput): Pending {
    const key = docKey(input.document) ?? randomUUID();
    const { embedding, ...rest } = input.document as Doc & { embedding?: unknown };
    const vec =
      Array.isArray(embedding) &&
      embedding.length === this.dim &&
      embedding.every(x => typeof x === 'number' && Number.isFinite(x))
        ? (embedding as number[]).slice()
        : undefined;
    const metadata: Record<string, unknown> = {
      collection: input.collection,
      docId: key,
      hasVector: vec ? 1 : 0,
      doc: JSON.stringify({ ...rest, id: key }),
    };
    if (vec) metadata.vec = encodeVector(vec);
    if (typeof rest.timestamp === 'number') metadata.timestamp = rest.timestamp;
    return {
      collection: input.collection,
      key,
      vec,
      metadata: (rest.metadata as Record<string, unknown>) ?? {},
      entry: { id: rvfId(input.collection, key), vector: vec ?? this.placeholder, metadata },
    };
  }

  /**
   * One batch = at most two rvf commits: a delete of the keys being
   * replaced, then one ingest. Re-ingesting an id that is still live would
   * make rvf drop its whole HNSW index, so replaced ids are deleted first.
   */
  private async commit(batch: Map<string, Pending>): Promise<void> {
    const entries = [...batch.values()];
    const replaced = entries.filter(p => this.onDisk.get(p.collection)?.has(p.key));
    try {
      if (replaced.length > 0) {
        await this.db.delete(replaced.map(p => p.entry.id));
        for (const p of replaced) this.onDisk.get(p.collection)?.delete(p.key);
      }
      const res = await this.db.ingestBatch(entries.map(p => p.entry));
      if (res.accepted !== entries.length) {
        throw new Error(
          `aidefence: rvf accepted ${res.accepted} of ${entries.length} documents`
        );
      }
      for (const p of entries) this.keys(this.onDisk, p.collection).add(p.key);
    } catch (error) {
      // Forget what did not reach the file, unless a newer write is queued.
      for (const p of entries) {
        const durable = this.onDisk.get(p.collection)?.has(p.key);
        if (!durable && !this.queue.isQueued(p.entry.id)) {
          this.index.get(p.collection)?.delete(p.key);
        }
      }
      throw error;
    }
  }

  private async deleteOlderThan(collection: string, cutoff: number): Promise<void> {
    const committed = this.onDisk.get(collection);
    if (!committed || committed.size === 0) return;
    const hits = await this.db.query(this.placeholder, committed.size, {
      forceExact: true,
      filter: {
        op: 'and',
        exprs: [
          { op: 'eq', field: 'collection', value: collection },
          { op: 'lt', field: 'timestamp', value: cutoff },
        ],
      },
    });
    const doomed = hits
      .map(h => h.metadata?.docId)
      .filter((k): k is string => typeof k === 'string' && committed.has(k));
    if (doomed.length === 0) return;
    await this.db.delete(doomed.map(k => rvfId(collection, k)));
    for (const k of doomed) {
      committed.delete(k);
      if (!this.queue.get(rvfId(collection, k))) this.index.get(collection)?.delete(k);
    }
  }

  /** Rebuild the per-collection key index from the file. */
  private async hydrate(): Promise<void> {
    const { totalVectors, fileSizeBytes } = await this.db.status();
    this.fileSizeBytes = fileSizeBytes;
    if (totalVectors === 0) return;
    const hits = await this.db.query(this.placeholder, totalVectors, { forceExact: true });
    for (const hit of hits) {
      const m = hit.metadata;
      if (m && typeof m.collection === 'string' && typeof m.docId === 'string') {
        this.keys(this.index, m.collection).add(m.docId);
        this.keys(this.onDisk, m.collection).add(m.docId);
      }
    }
  }

  private keys(map: Map<string, Set<string>>, collection: string): Set<string> {
    let set = map.get(collection);
    if (!set) {
      set = new Set();
      map.set(collection, set);
    }
    return set;
  }
}

/** One file holds every collection, so ids are namespaced by collection. */
function rvfId(collection: string, key: string): string {
  return `${collection}:${key}`;
}

function encodeVector(v: number[]): string {
  return Buffer.from(Float32Array.from(v).buffer).toString('base64');
}

function decodeVector(s: string): number[] {
  const buf = Buffer.from(s, 'base64');
  return Array.from(new Float32Array(buf.buffer, buf.byteOffset, buf.byteLength / 4));
}

/** Cosine similarity, matching rvf's cosine distance (similarity = 1 - distance). */
function cosine(a: number[], b: number[]): number {
  let dot = 0;
  let na = 0;
  let nb = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    na += a[i] * a[i];
    nb += b[i] * b[i];
  }
  const denom = Math.sqrt(na) * Math.sqrt(nb);
  return denom === 0 ? 0 : dot / denom;
}
