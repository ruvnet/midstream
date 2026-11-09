/**
 * Picks the storage backend for AgentDBClient.
 *
 * No path (or ':memory:') -> InMemoryStore; nothing touches the disk.
 * A path -> RvfStore at that file, if @ruvector/rvf can be loaded.
 * If the optional dependency is not installed, the client falls back
 * to InMemoryStore and logs a warning. Any other failure opening the
 * file (dimension mismatch, writer lock held) is thrown, so persistence
 * is never dropped silently.
 */

import { InMemoryStore } from './memory-store';
import { RvfModule, RvfStore } from './rvf-store';
import { StoreBackend } from './store-types';
import { AgentDBConfig } from '../types';
import { Logger } from '../utils/logger';

export type RvfLoader = () => Promise<RvfModule>;

/* The specifier is held in a variable so TypeScript does not need the
   optional package's types to compile this file. */
const RVF_PACKAGE = '@ruvector/rvf';
export const defaultRvfLoader: RvfLoader = () =>
  import(RVF_PACKAGE) as Promise<RvfModule>;

export function isPersistent(config: Pick<AgentDBConfig, 'path'>): boolean {
  return typeof config.path === 'string' && config.path !== '' && config.path !== ':memory:';
}

export async function createStore(
  config: AgentDBConfig,
  logger: Pick<Logger, 'info' | 'warn'>,
  loadRvf: RvfLoader = defaultRvfLoader
): Promise<StoreBackend> {
  if (!isPersistent(config)) return new InMemoryStore();

  let rvf: RvfModule;
  try {
    rvf = await loadRvf();
  } catch (error) {
    logger.warn(
      'AGENTDB path is set but @ruvector/rvf is not installed; using the in-memory store (nothing will persist)',
      { path: config.path, error: (error as Error).message }
    );
    return new InMemoryStore();
  }

  const store = await RvfStore.open(rvf, {
    path: config.path,
    dimension: config.embeddingDim,
    m: config.hnswConfig?.m,
    efConstruction: config.hnswConfig?.efConstruction,
    maxBatch: config.writeBehind?.maxBatch,
    flushIntervalMs: config.writeBehind?.flushIntervalMs,
    onWriteError: error =>
      logger.warn('aidefence: a background rvf commit failed', { error: error.message }),
  });
  logger.info('AgentDB using durable rvf store', { path: config.path, ...store.writeBehind });
  return store;
}
