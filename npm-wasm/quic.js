/**
 * `midstreamer/quic` — real QUIC transport for federation peers.
 *
 * Delegates to `agentic-flow/transport/loader`, which is the
 * production QUIC stack (UDP sockets, TLS, full handshake state
 * machine, 0-RTT reconnection). Both packages share one validated
 * transport implementation — no parallel stub.
 *
 * Performance baseline (from `agentic-flow/docs/features/quic/
 * QUIC-STATUS.md`):
 *   - 53.7% lower latency than HTTP/2
 *   - 91.2% improvement on 0-RTT reconnection
 *   - 7931 MB/s throughput under stream multiplexing
 *
 * Surface matches `agentic-flow/transport/loader` exactly so
 * downstream consumers (e.g. ruvnet/ruflo's federation transport
 * loader) can swap between the two with a single env flag.
 *
 * Importable without WASM init — this module has no dependency on
 * the `@midstream/wasm` bindings. Use `midstreamer` (default
 * export) for the temporal/scheduling/meta-learning surface that
 * does require WASM.
 *
 * @module midstreamer/quic
 */

'use strict';

/**
 * Load a real QUIC transport.
 *
 * @param {Object} [config] - QuicTransportConfig
 *   ({serverName, maxIdleTimeoutMs, maxConcurrentStreams,
 *     enable0Rtt, tls}).
 * @returns {Promise<import('agentic-flow/transport/loader').AgentTransport>}
 */
async function loadQuicTransport(config) {
  const mod = await import('agentic-flow/transport/loader');
  return mod.loadQuicTransport(config);
}

/**
 * Probe whether QUIC is available without instantiating a transport.
 * Returns true when `agentic-flow/transport/loader` reports ready.
 * @returns {Promise<boolean>}
 */
async function isQuicAvailable() {
  try {
    const mod = await import('agentic-flow/transport/loader');
    return typeof mod.isQuicAvailable === 'function'
      ? await mod.isQuicAvailable()
      : true;
  } catch {
    return false;
  }
}

/**
 * Reports that this loader is backed by the real agentic-flow QUIC
 * stack rather than a stub. Used by ruvnet/ruflo's federation
 * transport loader to confirm before binding.
 * @returns {boolean}
 */
function isNative() {
  return true;
}

module.exports = { loadQuicTransport, isQuicAvailable, isNative };
module.exports.default = module.exports;
