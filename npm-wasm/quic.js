/**
 * `midstreamer/quic` — federation transport entry point.
 *
 * Delegates to `agentic-flow/transport/loader` (optional peer
 * dependency). As of agentic-flow 3.5.4 that loader has no native QUIC
 * binding: the quinn-based crate has no N-API build and the bundled
 * WASM QUIC module is a browser stub without UDP. `loadQuicTransport()`
 * therefore returns agentic-flow's WebSocket fallback transport, and
 * agentic-flow's own `getQuicStatus()` reports `available: false`.
 *
 * This module does not implement QUIC itself and makes no QUIC
 * performance claims. `isQuicAvailable()` reports what agentic-flow
 * reports; `isNative()` is false because no native QUIC binding ships.
 *
 * Importable without WASM init — this module has no dependency on the
 * WASM bindings. Use `midstreamer` (default export) for the
 * temporal/scheduling/meta-learning surface that does require WASM.
 *
 * @module midstreamer/quic
 */

'use strict';

/**
 * Load a transport through agentic-flow's transport loader. Today this
 * is the WebSocket fallback (see module comment); the loader will pick
 * up native QUIC without a code change here if agentic-flow ships it.
 *
 * @param {Object} [config] - QuicTransportConfig
 *   ({serverName, maxIdleTimeoutMs, maxConcurrentStreams,
 *     enable0Rtt, tls}).
 * @returns {Promise<import('agentic-flow/transport/loader').AgentTransport>}
 */
async function loadQuicTransport(config) {
  // `agentic-flow` is an optional peer dependency as of
  // midstreamer@0.3.2 — transport consumers install it explicitly;
  // temporal-compare consumers never pull its dependency tree.
  let mod;
  try {
    mod = await import('agentic-flow/transport/loader');
  } catch (error) {
    throw new Error(
      "midstreamer/quic: the transport requires the optional peer dependency 'agentic-flow'. " +
      'Install it with `npm install agentic-flow` (or use isQuicAvailable() to probe first).',
      { cause: error }
    );
  }
  return mod.loadQuicTransport(config);
}

/**
 * Whether real QUIC is available. Resolves to agentic-flow's own
 * `isQuicAvailable()` answer (false in every published agentic-flow
 * release to date), or false when agentic-flow is not installed or does
 * not expose the probe.
 * @returns {Promise<boolean>}
 */
async function isQuicAvailable() {
  try {
    const mod = await import('agentic-flow/transport/loader');
    return typeof mod.isQuicAvailable === 'function'
      ? (await mod.isQuicAvailable()) === true
      : false;
  } catch {
    return false;
  }
}

/**
 * Whether this loader is backed by a native QUIC binding. Always false:
 * no native QUIC binding is published, so the transport returned by
 * `loadQuicTransport()` is agentic-flow's WebSocket fallback.
 * @returns {boolean}
 */
function isNative() {
  return false;
}

module.exports = { loadQuicTransport, isQuicAvailable, isNative };
module.exports.default = module.exports;
