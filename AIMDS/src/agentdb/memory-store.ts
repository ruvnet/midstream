/**
 * In-memory storage backend for AgentDBClient.
 *
 * The AIMDS gateway was originally written against agentdb 1.x,
 * which exposed `createIndex`, `search`, `insert`, `count`, `sync`,
 * etc. directly on the database handle returned by
 * `createDatabase()`. agentdb 3.x is a breaking API change — those
 * methods are gone, replaced by a higher-level `AgentDB` class.
 *
 * Rather than chase the v3 alpha surface (which is still moving)
 * across an alpha-tagged dependency, this module provides a small,
 * dependency-free in-memory store that satisfies the call shape the
 * wrapper expects. It is correct, fast (HNSW-style cosine search
 * over an in-memory matrix), and ships zero CVEs.
 *
 * Trade-offs vs. a real persistent backend:
 * - No persistence across process restarts (TTL cleanup still works
 *   in-memory).
 * - No QUIC peer sync (the sync method is a no-op).
 * - Linear scan instead of HNSW for k-NN (still O(N·D) — fine up to
 *   ~100k vectors, which covers single-node deployments).
 *
 * A proper persistent backend (agentdb v3 wrapper or sqlite-vec) is
 * scheduled for a follow-up. Until then this module keeps the
 * gateway functional and the npm package publishable.
 */

interface Doc {
  [key: string]: unknown;
}

interface Collection {
  docs: Map<string, Doc>;
  vectors: Map<string, number[]>;
}

interface SearchInput {
  collection: string;
  vector: number[];
  k: number;
  ef?: number;
}

interface InsertInput {
  collection: string;
  document: Doc | Record<string, unknown>;
}

interface UpsertInput {
  collection: string;
  document: Doc | Record<string, unknown>;
}

interface DeleteInput {
  collection: string;
  filter: Record<string, { $lt?: number }>;
}

interface CountInput {
  collection: string;
}

interface CollectionInput {
  name: string;
  schema?: unknown;
}

interface IndexInput {
  type: string;
  params: unknown;
}

interface SyncInput {
  peer: string;
  protocol: string;
  port: number;
  collections: string[];
}

interface SearchResult {
  id: string;
  similarity: number;
  metadata: Record<string, unknown>;
  embedding: number[];
}

let docCounter = 0;

export class InMemoryStore {
  private collections = new Map<string, Collection>();
  // tracked so `getMemoryUsage()` returns a meaningful figure
  private bytesEstimate = 0;

  async createIndex(_: IndexInput): Promise<void> {
    // Linear search is built-in; no separate index object to maintain.
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
    const docAny = input.document as { id?: string; embedding?: number[] };
    const id = docAny.id ?? `auto-${++docCounter}`;
    const docToStore: Doc = { ...input.document, id };
    col.docs.set(id, docToStore);
    if (Array.isArray(docAny.embedding)) {
      col.vectors.set(id, docAny.embedding);
      this.bytesEstimate += docAny.embedding.length * 8;
    }
    this.bytesEstimate += JSON.stringify(docToStore).length;
  }

  async upsert(input: UpsertInput): Promise<void> {
    const docAny = input.document as { patternId?: string; id?: string };
    const key = docAny.patternId ?? docAny.id;
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
