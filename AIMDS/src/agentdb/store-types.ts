/**
 * The storage contract AgentDBClient is written against. Two backends
 * implement it: InMemoryStore (default, a Map with a linear cosine scan)
 * and RvfStore (opt-in, a durable @ruvector/rvf file with an HNSW index).
 */

export interface Doc {
  [key: string]: unknown;
}

export interface SearchInput {
  collection: string;
  vector: number[];
  k: number;
  ef?: number;
}

export interface InsertInput {
  collection: string;
  document: Doc | Record<string, unknown>;
}

export type UpsertInput = InsertInput;

export interface DeleteInput {
  collection: string;
  filter: Record<string, { $lt?: number }>;
}

export interface CountInput {
  collection: string;
}

export interface CollectionInput {
  name: string;
  schema?: unknown;
}

export interface IndexInput {
  type: string;
  params: unknown;
}

export interface SyncInput {
  peer: string;
  protocol: string;
  port: number;
  collections: string[];
}

export interface SearchResult {
  id: string;
  similarity: number;
  metadata: Record<string, unknown>;
  embedding: number[];
}

export interface StoreBackend {
  /** 'memory' or 'rvf'; lets callers and tests see which backend is live. */
  readonly kind: 'memory' | 'rvf';
  createIndex(input: IndexInput): Promise<void>;
  createCollection(input: CollectionInput): Promise<void>;
  insert(input: InsertInput): Promise<void>;
  upsert(input: UpsertInput): Promise<void>;
  search(input: SearchInput): Promise<SearchResult[]>;
  count(input: CountInput): Promise<number>;
  getMemoryUsage(): number;
  delete(input: DeleteInput): Promise<void>;
  sync(input: SyncInput): Promise<void>;
  close(): Promise<void>;
}

/** The key a document is stored under: `id`, else `patternId`. */
export function docKey(doc: Record<string, unknown>): string | undefined {
  const id = doc.id ?? doc.patternId;
  return typeof id === 'string' && id.length > 0 ? id : undefined;
}
