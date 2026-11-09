/**
 * aidefence library entry. Importing this module has no side effects: it
 * starts no server and opens no files. Run the gateway with `npx aidefence`
 * (dist/server.js).
 */

export * from './detection';
export { AIMDSGateway } from './gateway/server';
export * from './types';
export { AgentDBClient } from './agentdb/client';
export { InMemoryStore } from './agentdb/memory-store';
export { RvfStore } from './agentdb/rvf-store';
export { createStore, isPersistent } from './agentdb/store-factory';
export type { RvfLoader } from './agentdb/store-factory';
export type { StoreBackend } from './agentdb/store-types';
export { Logger } from './utils/logger';
