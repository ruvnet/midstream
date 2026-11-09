/**
 * Durable storage backend for AgentDBClient, on @ruvector/rvf.
 *
 * All collections share one `.rvf` file. Each document is one vector
 * whose per-vector metadata carries the collection name, the document
 * key and the document itself (JSON), so a reopen restores everything
 * without a separate sidecar. Search is rvf's HNSW index, scoped to a
 * collection with a name-based metadata filter.
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

/** The parts of the @ruvector/rvf surface this backend uses. */
type FilterExpr =
  | { op: 'eq' | 'lt'; field: string; value: string | number | boolean }
  | { op: 'and'; exprs: FilterExpr[] };

interface RvfHit {
  id: string;
  distance: number;
  metadata?: Record<string, unknown>;
}

export interface RvfDatabaseLike {
  ingestBatch(
    entries: { id: string; vector: number[] | Float32Array; metadata?: Record<string, unknown> }[]
  ): Promise<{ accepted: number; rejected: number }>;
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

export interface RvfStoreOptions {
  path: string;
  dimension: number;
  m?: number;
  efConstruction?: number;
}

export class RvfStore implements StoreBackend {
  readonly kind = 'rvf' as const;
  /** collection -> document keys currently stored */
  private index = new Map<string, Set<string>>();
  private fileSizeBytes = 0;
  /** Stands in for the vector of a document that has none of the right size. */
  private readonly placeholder: number[];

  private constructor(private db: RvfDatabaseLike, private dim: number) {
    this.placeholder = new Array(dim).fill(0);
    this.placeholder[0] = 1;
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
    const store = new RvfStore(db, opts.dimension);
    await store.hydrate();
    return store;
  }

  async createIndex(_: IndexInput): Promise<void> {
    // The HNSW index is built by rvf as vectors are ingested.
  }

  async createCollection(input: CollectionInput): Promise<void> {
    this.keys(input.name);
  }

  async insert(input: InsertInput): Promise<void> {
    const key = docKey(input.document) ?? randomUUID();
    const keys = this.keys(input.collection);
    // A second write to the same key replaces the first, as in InMemoryStore.
    if (keys.has(key)) await this.db.delete([rvfId(input.collection, key)]);

    const { embedding, ...rest } = input.document as Doc & { embedding?: unknown };
    const vec = Array.isArray(embedding) && embedding.length === this.dim
      ? (embedding as number[])
      : undefined;
    const metadata: Record<string, unknown> = {
      collection: input.collection,
      docId: key,
      hasVector: vec ? 1 : 0,
      doc: JSON.stringify({ ...rest, id: key }),
    };
    if (vec) metadata.vec = encodeVector(vec);
    if (typeof rest.timestamp === 'number') metadata.timestamp = rest.timestamp;

    const res = await this.db.ingestBatch([
      { id: rvfId(input.collection, key), vector: vec ?? this.placeholder, metadata },
    ]);
    if (res.accepted !== 1) {
      throw new Error(`aidefence: rvf rejected document ${input.collection}/${key}`);
    }
    keys.add(key);
  }

  async upsert(input: UpsertInput): Promise<void> {
    return this.insert(input);
  }

  async search(input: SearchInput): Promise<SearchResult[]> {
    const keys = this.index.get(input.collection);
    if (!keys || keys.size === 0 || input.vector.length !== this.dim) return [];

    const hits = await this.db.query(input.vector, Math.min(input.k, keys.size), {
      efSearch: input.ef,
      filter: {
        op: 'and',
        exprs: [
          { op: 'eq', field: 'collection', value: input.collection },
          { op: 'eq', field: 'hasVector', value: 1 },
        ],
      },
    });

    const out: SearchResult[] = [];
    for (const hit of hits) {
      const m = hit.metadata;
      if (!m || typeof m.docId !== 'string' || !keys.has(m.docId)) continue;
      const doc = JSON.parse(String(m.doc)) as Doc;
      out.push({
        id: m.docId,
        similarity: 1 - hit.distance,
        embedding: typeof m.vec === 'string' ? decodeVector(m.vec) : [],
        metadata: (doc.metadata as Record<string, unknown>) ?? {},
      });
    }
    return out;
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
    const keys = this.index.get(input.collection);
    const cutoff = input.filter.timestamp?.$lt;
    if (!keys || keys.size === 0 || cutoff === undefined) return;

    const hits = await this.db.query(this.placeholder, keys.size, {
      forceExact: true,
      filter: {
        op: 'and',
        exprs: [
          { op: 'eq', field: 'collection', value: input.collection },
          { op: 'lt', field: 'timestamp', value: cutoff },
        ],
      },
    });
    const doomed = hits
      .map(h => h.metadata?.docId)
      .filter((k): k is string => typeof k === 'string' && keys.has(k));
    if (doomed.length === 0) return;
    await this.db.delete(doomed.map(k => rvfId(input.collection, k)));
    for (const k of doomed) keys.delete(k);
  }

  async sync(_: SyncInput): Promise<void> {
    // Peer sync is not implemented for either backend.
  }

  async close(): Promise<void> {
    await this.db.close();
    this.index.clear();
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
        this.keys(m.collection).add(m.docId);
      }
    }
  }

  private keys(collection: string): Set<string> {
    let set = this.index.get(collection);
    if (!set) {
      set = new Set();
      this.index.set(collection, set);
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
