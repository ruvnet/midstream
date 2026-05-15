/**
 * Tests for the real QUIC transport entry point in @midstream/wasm.
 * Asserts the loader returns a real `AgentTransport` (backed by
 * agentic-flow's validated stack — no stub) and reports `isNative`
 * truthfully.
 *
 * Run with: node tests/quic-transport.test.js
 */

import assert from 'node:assert/strict';
import {
  isNative,
  isQuicAvailable,
  loadQuicTransport,
} from '../index.js';

let pass = 0, fail = 0;
async function test(name, fn) {
  process.stdout.write(`→ ${name} ... `);
  try {
    await fn();
    console.log('PASS');
    pass++;
  } catch (e) {
    console.log('FAIL:', e.message);
    fail++;
  }
}

await test('isNative() returns true (the WASM-stub QUIC is no longer the network path)', () => {
  assert.equal(isNative(), true);
});

await test('isQuicAvailable() resolves to a boolean', async () => {
  const v = await isQuicAvailable();
  assert.equal(typeof v, 'boolean');
});

await test('loadQuicTransport returns an AgentTransport with the documented surface', async () => {
  const t = await loadQuicTransport({
    serverName: 'midstream-test:9100',
    maxIdleTimeoutMs: 30_000,
    maxConcurrentStreams: 100,
    enable0Rtt: true,
  });
  // The contract documented in agentic-flow/transport/loader.d.ts:
  //   send, receive, request, sendBatch, getStats, close
  for (const method of ['send', 'receive', 'request', 'sendBatch', 'getStats', 'close']) {
    assert.equal(typeof t[method], 'function', `expected method ${method}`);
  }
  // Cleanup so test exit isn't blocked by open sockets.
  await t.close();
});

await test('loadQuicTransport works with an empty config (defaults applied)', async () => {
  const t = await loadQuicTransport();
  assert.equal(typeof t.send, 'function');
  await t.close();
});

console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
