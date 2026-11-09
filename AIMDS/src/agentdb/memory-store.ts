/**
 * In-memory storage backend for AgentDBClient (the default).
 *
 * Documents live in a Map per collection and k-NN search is an exact
 * linear cosine scan, O(N·D) per query. Nothing is persisted: a restart
 * loses every threat pattern, incident and reflexion entry, and `sync`
 * is a no-op. Set `AgentDBConfig.path` to use the durable RvfStore
 * (./rvf-store.ts) instead.
 */

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

interface Collection {
  docs: Map<string, Doc>;
  vectors: Map<string, number[]>;
}

let docCounter = 0;

export class InMemoryStore implements StoreBackend {
  readonly kind = 'memory' as const;
  private collections = new Map<string, Collection>();
  // tracked so `getMemoryUsage()` returns a meaningful figure
  private bytesEstimate = 0;

  async createIndex(_: IndexInput): Promise<void> {
    // No index: search is an exact linear scan.
  }

  async createCollection(input: CollectionInput): Promise<void> {
    if (!this.collections.has(input.name)) {
      this.collections.set(input.name, {
        docs: new Map(),
        vectors: new Map(),
      });
    }
  }

  async insert(input: InsertInput): Promise<void> {
    const col = this.ensure(input.collection);
    const docAny = input.document as { embedding?: number[] };
    const id = docKey(input.document) ?? `auto-${++docCounter}`;
    const docToStore: Doc = { ...input.document, id };
    col.docs.set(id, docToStore);
    if (Array.isArray(docAny.embedding)) {
      col.vectors.set(id, docAny.embedding);
      this.bytesEstimate += docAny.embedding.length * 8;
    }
    this.bytesEstimate += JSON.stringify(docToStore).length;
  }

  async insertMany(inputs: InsertInput[]): Promise<void> {
    for (const input of inputs) await this.insert(input);
  }

  async flush(): Promise<void> {
    // Nothing to persist.
  }

  async upsert(input: UpsertInput): Promise<void> {
    const key = docKey(input.document);
    if (key) {
      const col = this.ensure(input.collection);
      col.docs.delete(key);
      col.vectors.delete(key);
    }
    return this.insert({ collection: input.collection, document: input.document });
  }

  async search(input: SearchInput): Promise<SearchResult[]> {
    const col = this.collections.get(input.collection);
    if (!col || col.vectors.size === 0) return [];

    const ranked: SearchResult[] = [];
    for (const [id, vec] of col.vectors) {
      const sim = cosineSimilarity(input.vector, vec);
      const doc = col.docs.get(id) ?? {};
      ranked.push({
        id,
        similarity: sim,
        embedding: vec,
        metadata: (doc.metadata as Record<string, unknown>) ?? {},
      });
    }
    ranked.sort((a, b) => b.similarity - a.similarity);
    return ranked.slice(0, input.k);
  }

  async count(input: CountInput): Promise<number> {
    return this.collections.get(input.collection)?.docs.size ?? 0;
  }

  getMemoryUsage(): number {
    return this.bytesEstimate;
  }

  async delete(input: DeleteInput): Promise<void> {
    const col = this.collections.get(input.collection);
    if (!col) return;
    const tsFilter = input.filter.timestamp?.$lt;
    if (tsFilter !== undefined) {
      for (const [id, doc] of col.docs) {
        const ts = (doc.timestamp as number | undefined) ?? Infinity;
        if (ts < tsFilter) {
          col.docs.delete(id);
          col.vectors.delete(id);
        }
      }
    }
  }

  async sync(_: SyncInput): Promise<void> {
    // No-op until a persistent / distributed backend is wired.
  }

  async close(): Promise<void> {
    this.collections.clear();
    this.bytesEstimate = 0;
  }

  private ensure(name: string): Collection {
    let col = this.collections.get(name);
    if (!col) {
      col = { docs: new Map(), vectors: new Map() };
      this.collections.set(name, col);
    }
    return col;
  }
}

function cosineSimilarity(a: number[], b: number[]): number {
  if (a.length !== b.length || a.length === 0) return 0;
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
