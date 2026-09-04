/**
 * Gateway integration: payload text is scanned by the pattern packs and the
 * finding feeds the existing allow/deny decision.
 *
 * Deny semantics are the gateway's own and unchanged:
 *   allowed = verifier.valid && threatLevel < CRITICAL
 * so a critical finding is denied (403) while a high finding is routed
 * through the verifier and allowed if the verifier accepts the action.
 */

import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';
import request from 'supertest';
import { readFileSync } from 'fs';
import { join } from 'path';

// London-school: the unit under test is the gateway's wiring of the pattern
// packs into its allow/deny decision. The vector store and the lean-agentic
// verifier (and the prom-client metrics registry, which is process-global)
// are collaborators and are stubbed (the real verifier also cannot
// load in this repo's test environment: lean-agentic's wasm file is absent
// from the npm package, which is why tests/integration/gateway.test.ts fails
// on main).
vi.mock('../../src/agentdb/client', () => ({
  AgentDBClient: class {
    async initialize(): Promise<void> {}
    async vectorSearch(): Promise<unknown[]> { return []; }
    async storeIncident(): Promise<void> {}
    async getStats(): Promise<Record<string, unknown>> { return { totalPatterns: 0 }; }
    async shutdown(): Promise<void> {}
  },
}));
vi.mock('../../src/monitoring/metrics', () => ({
  MetricsCollector: class {
    async initialize(): Promise<void> {}
    recordDetection(): void {}
    recordRequest(): void {}
    recordError(): void {}
    async exportPrometheus(): Promise<string> { return '# stub\n'; }
    async getStats(): Promise<Record<string, unknown>> { return {}; }
    async getMetrics(): Promise<Record<string, unknown>> { return {}; }
    getSnapshot(): Record<string, unknown> { return {}; }
    async shutdown(): Promise<void> {}
  },
}));
vi.mock('../../src/lean-agentic/verifier', () => ({
  LeanAgenticVerifier: class {
    async initialize(): Promise<void> {}
    async verifyPolicy(): Promise<{ valid: boolean; proof: { id: string } }> { return { valid: true, proof: { id: 'stub-proof' } }; }
    getCacheStats(): Record<string, unknown> { return { size: 0 }; }
    async shutdown(): Promise<void> {}
  },
}));

import { AIMDSGateway } from '../../src/gateway/server';
import { Config } from '../../src/utils/config';

const corpus: { cases: { id: string; text: string }[] } = JSON.parse(
  readFileSync(join(__dirname, '..', 'fixtures', 'injection-corpus.json'), 'utf8'),
);
const text = (id: string) => corpus.cases.find((c) => c.id === id)!.text;

describe('Gateway injection scanning', () => {
  let gateway: AIMDSGateway;
  let app: any;

  beforeAll(async () => {
    const config = Config.getInstance();
    gateway = new AIMDSGateway(
      { ...config.getGatewayConfig(), port: 3002 },
      config.getAgentDBConfig(),
      config.getLeanAgenticConfig(),
    );
    await gateway.initialize();
    app = (gateway as any).app;
  });

  afterAll(async () => {
    await gateway.shutdown();
  });

  const defend = (payload: unknown) =>
    request(app)
      .post('/api/v1/defend')
      .send({ action: { type: 'read', resource: '/docs', method: 'GET', payload }, source: { ip: '10.0.0.1' } });

  it('denies a payload carrying a critical instruction override (C01)', async () => {
    const res = await defend({ text: text('C01') }).expect(403);
    expect(res.body.allowed).toBe(false);
    expect(res.body.threatLevel).toBe('CRITICAL');
  });

  it('denies a base64-encoded override once decoded (E01)', async () => {
    const res = await defend({ page: { body: text('E01') } }).expect(403);
    expect(res.body.allowed).toBe(false);
  });

  it('detects a high tool-invocation directive (T01), routes it through the verifier, and allows it when the verifier accepts', async () => {
    const res = await defend({ text: text('T01') }).expect(200);
    // Detected: the request leaves the fast path. Not denied: TI-001 is high, not critical,
    // and allowed = verifier.valid && threatLevel < CRITICAL.
    expect(res.body.metadata.pathTaken).toBe('deep');
    expect(res.body.threatLevel).toBe('HIGH');
    expect(res.body.allowed).toBe(true);
  });

  it('scans request context as well as payload', async () => {
    const res = await request(app)
      .post('/api/v1/defend')
      .send({ action: { type: 'read', resource: '/docs', method: 'GET' }, source: { ip: '10.0.0.1' }, context: { fetched: text('C03') } })
      .expect(403);
    expect(res.body.threatLevel).toBe('CRITICAL');
  });

  it('can be disabled per gateway config', async () => {
    const config = Config.getInstance();
    const off = new AIMDSGateway(
      { ...config.getGatewayConfig(), port: 3003, injectionDetection: { enabled: false } },
      config.getAgentDBConfig(),
      config.getLeanAgenticConfig(),
    );
    await off.initialize();
    const res = await request((off as any).app)
      .post('/api/v1/defend')
      .send({ action: { type: 'read', resource: '/docs', method: 'GET', payload: { text: text('C01') } }, source: { ip: '10.0.0.1' } })
      .expect(200);
    expect(res.body.allowed).toBe(true);
    await off.shutdown();
  });

  it('allows a benign payload with a normal URL (F05)', async () => {
    const res = await defend({ text: text('F05') }).expect(200);
    expect(res.body.allowed).toBe(true);
    expect(res.body.threatLevel).toBe('NONE');
  });

  it('allows a request without any text payload, as before', async () => {
    const res = await defend(undefined).expect(200);
    expect(res.body.allowed).toBe(true);
  });
});
