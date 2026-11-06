/**
 * midstreamer - WebAssembly bindings for Midstream
 *
 * Browser and Node.js compatible wrapper for temporal comparison,
 * nanosecond scheduling, meta-learning, and QUIC multistream.
 *
 * This file is CommonJS (the package has no `"type": "module"`), so
 * `require('midstreamer')` works directly and `import 'midstreamer'`
 * gets named exports via Node's CJS named-export detection.
 */

'use strict';

let wasm;
let initialized = false;

/**
 * Initialize the WASM module
 * @param {string} [wasmPath] - Optional custom path to WASM file
 * @returns {Promise<void>}
 */
async function init(wasmPath) {
  if (initialized) {
    return;
  }

  try {
    // Detect environment
    const isNode = typeof process !== 'undefined' && process.versions && process.versions.node;
    const isBrowser = typeof window !== 'undefined';

    if (isBrowser) {
      // Browser environment - use bundler target (works in browsers)
      const wasmModule = await import('./pkg-bundler/midstream_wasm.js');
      await wasmModule.default();
      wasm = wasmModule;
    } else if (isNode) {
      // Node.js environment - use nodejs target (package.json uses pkg-node)
      const wasmModule = await import('./pkg-node/midstream_wasm.js');
      wasm = wasmModule;
    } else {
      throw new Error('Unsupported environment');
    }

    initialized = true;
    console.log('[Midstream WASM] Initialized successfully');
  } catch (error) {
    console.error('[Midstream WASM] Initialization failed:', error);
    throw error;
  }
}

/**
 * Ensure WASM is initialized
 * @private
 */
function ensureInitialized() {
  if (!initialized) {
    throw new Error('WASM not initialized. Call init() first.');
  }
}

// ============================================================================
// TEMPORAL COMPARISON API
// ============================================================================

/**
 * Temporal comparison utilities (DTW, LCS, Edit Distance)
 */
class TemporalCompare {
  constructor(windowSize = 100) {
    ensureInitialized();
    this.instance = new wasm.TemporalCompare(windowSize);
  }

  /**
   * Calculate Dynamic Time Warping distance
   * @param {number[]} seq1 - First sequence
   * @param {number[]} seq2 - Second sequence
   * @returns {number} DTW distance
   */
  dtw(seq1, seq2) {
    return this.instance.dtw(new Float64Array(seq1), new Float64Array(seq2));
  }

  /**
   * Calculate Longest Common Subsequence length
   * @param {number[]} seq1 - First sequence
   * @param {number[]} seq2 - Second sequence
   * @returns {number} LCS length
   */
  lcs(seq1, seq2) {
    return this.instance.lcs(new Int32Array(seq1), new Int32Array(seq2));
  }

  /**
   * Calculate Levenshtein edit distance
   * @param {string} s1 - First string
   * @param {string} s2 - Second string
   * @returns {number} Edit distance
   */
  editDistance(s1, s2) {
    return this.instance.edit_distance(s1, s2);
  }

  /**
   * Comprehensive temporal analysis
   * @param {number[]} seq1 - First sequence
   * @param {number[]} seq2 - Second sequence
   * @returns {Object} Analysis results with dtw, lcs, edit distance, and similarity
   */
  analyze(seq1, seq2) {
    const result = this.instance.analyze(new Float64Array(seq1), new Float64Array(seq2));
    return {
      dtwDistance: result.dtw_distance,
      lcsLength: result.lcs_length,
      editDistance: result.edit_distance,
      similarityScore: result.similarity_score
    };
  }
}

// ============================================================================
// NANOSECOND SCHEDULER API
// ============================================================================

/**
 * Nanosecond-precision task scheduler
 */
class NanoScheduler {
  constructor() {
    ensureInitialized();
    this.instance = new wasm.NanoScheduler();
    this.animationFrameId = null;
    this.running = false;
  }

  /**
   * Schedule a one-time task
   * @param {Function} callback - Function to execute
   * @param {number} delayNs - Delay in nanoseconds
   * @returns {number} Task ID
   */
  schedule(callback, delayNs) {
    return this.instance.schedule(callback, delayNs);
  }

  /**
   * Schedule a repeating task
   * @param {Function} callback - Function to execute
   * @param {number} intervalNs - Interval in nanoseconds
   * @returns {number} Task ID
   */
  scheduleRepeating(callback, intervalNs) {
    return this.instance.schedule_repeating(callback, intervalNs);
  }

  /**
   * Cancel a scheduled task
   * @param {number} taskId - Task ID to cancel
   * @returns {boolean} Success status
   */
  cancel(taskId) {
    return this.instance.cancel(taskId);
  }

  /**
   * Get current time in nanoseconds
   * @returns {number} Current time in nanoseconds
   */
  nowNs() {
    return this.instance.now_ns();
  }

  /**
   * Start the scheduler (begins processing tasks)
   */
  start() {
    if (this.running) return;
    this.running = true;

    const tick = () => {
      if (!this.running) return;
      this.instance.tick();
      this.animationFrameId = requestAnimationFrame(tick);
    };

    tick();
  }

  /**
   * Stop the scheduler
   */
  stop() {
    this.running = false;
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
  }

  /**
   * Get number of pending tasks
   * @returns {number} Pending task count
   */
  get pendingCount() {
    return this.instance.pending_count;
  }
}

// ============================================================================
// STRANGE LOOP META-LEARNING API
// ============================================================================

/**
 * Meta-learning and pattern recognition
 */
class StrangeLoop {
  constructor(learningRate = 0.1) {
    ensureInitialized();
    this.instance = new wasm.StrangeLoop(learningRate);
  }

  /**
   * Observe a pattern and learn from it
   * @param {string} patternId - Pattern identifier
   * @param {number} performance - Performance metric (0.0 to 1.0)
   */
  observe(patternId, performance) {
    this.instance.observe(patternId, performance);
  }

  /**
   * Get confidence for a pattern
   * @param {string} patternId - Pattern identifier
   * @returns {number|null} Confidence score (0.0 to 1.0)
   */
  getConfidence(patternId) {
    return this.instance.get_confidence(patternId);
  }

  /**
   * Get the best pattern learned so far
   * @returns {Object|null} Best pattern with id, confidence, iteration, improvement
   */
  bestPattern() {
    const pattern = this.instance.best_pattern();
    if (!pattern) return null;

    return {
      patternId: pattern.pattern_id,
      confidence: pattern.confidence,
      iteration: pattern.iteration,
      improvement: pattern.improvement
    };
  }

  /**
   * Reflect on learning progress (meta-cognition)
   * @returns {Object} All learned patterns
   */
  reflect() {
    return this.instance.reflect();
  }

  /**
   * Get iteration count
   * @returns {number} Total iterations
   */
  get iterationCount() {
    return this.instance.iteration_count;
  }

  /**
   * Get pattern count
   * @returns {number} Number of learned patterns
   */
  get patternCount() {
    return this.instance.pattern_count;
  }
}

// ============================================================================
// QUIC MULTISTREAM API
// ============================================================================

/**
 * QUIC multistream (WebTransport compatible)
 */
class QuicMultistream {
  constructor() {
    ensureInitialized();
    this.instance = new wasm.QuicMultistream();
  }

  /**
   * Open a new stream with priority
   * @param {number} priority - Stream priority (0-255)
   * @returns {number} Stream ID
   */
  openStream(priority = 128) {
    return this.instance.open_stream(priority);
  }

  /**
   * Close a stream
   * @param {number} streamId - Stream ID
   * @returns {boolean} Success status
   */
  closeStream(streamId) {
    return this.instance.close_stream(streamId);
  }

  /**
   * Send data on a stream
   * @param {number} streamId - Stream ID
   * @param {Uint8Array} data - Data to send
   * @returns {number} Bytes sent
   */
  send(streamId, data) {
    return this.instance.send(streamId, data);
  }

  /**
   * Receive data on a stream
   * @param {number} streamId - Stream ID
   * @param {number} size - Bytes to receive
   * @returns {Uint8Array} Received data
   */
  receive(streamId, size) {
    return this.instance.receive(streamId, size);
  }

  /**
   * Get stream statistics
   * @param {number} streamId - Stream ID
   * @returns {Object} Stream stats
   */
  getStats(streamId) {
    return this.instance.get_stats(streamId);
  }

  /**
   * Get stream count
   * @returns {number} Number of active streams
   */
  get streamCount() {
    return this.instance.stream_count;
  }
}

// ============================================================================
// QUIC TRANSPORT (real UDP/TLS via agentic-flow's validated stack)
// ============================================================================

/**
 * Load a real QUIC transport. This is the production network entry
 * point — UDP sockets, TLS, full handshake state machine, 0-RTT
 * reconnection, validated end-to-end (53.7% lower latency vs HTTP/2,
 * 91.2% reconnection improvement per `agentic-flow/docs/features/quic/
 * QUIC-STATUS.md`).
 *
 * The underlying stack is `agentic-flow/transport/loader`. midstream
 * delegates so the two packages share one validated QUIC implementation
 * — no parallel stub, no duplicate maintenance surface.
 *
 * Returns an `AgentTransport` directly (`send` / `receive` / `request`
 * / `sendBatch` / `getStats` / `close`), matching the
 * `agentic-flow/transport/loader` contract exactly so downstream
 * consumers (e.g. ruvnet/ruflo's federation transport loader) can
 * treat the two interchangeably.
 *
 * @param {Object} [config] - QuicTransportConfig
 *   ({serverName, maxIdleTimeoutMs, maxConcurrentStreams, enable0Rtt, tls})
 * @returns {Promise<import('agentic-flow/transport/loader').AgentTransport>}
 */
async function loadQuicTransport(config) {
  // Dynamic import so the dep stays out of bundles that never touch
  // QUIC. `agentic-flow` is an *optional peer dependency* as of
  // midstreamer@0.3.2 — temporal-compare consumers don't pull its
  // ~580-package tree. QUIC consumers install it explicitly.
  let mod;
  try {
    mod = await import('agentic-flow/transport/loader');
  } catch (error) {
    throw new Error(
      "midstreamer: QUIC transport requires the optional peer dependency 'agentic-flow'. " +
      'Install it with `npm install agentic-flow` (or use isQuicAvailable() to probe first).',
      { cause: error }
    );
  }
  return mod.loadQuicTransport(config);
}

/**
 * Probe whether QUIC is available without instantiating a transport.
 * Returns true when `agentic-flow/transport/loader` exposes a working
 * `isQuicAvailable()` AND the underlying stack reports ready, false
 * otherwise (e.g. browser bundle where the loader isn't resolvable).
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
 * Reports that this `loadQuicTransport` is backed by the real
 * agentic-flow QUIC stack (UDP sockets, TLS, full handshake) rather
 * than a counter-tracking stub. Consumers (e.g. ruvnet/ruflo's
 * federation transport loader) use this to confirm they're not
 * binding a stubbed network path.
 *
 * The synchronous flavor; for the equivalent async probe that also
 * checks the backend is loadable, see `isQuicAvailable()`.
 * @returns {boolean}
 */
function isNative() {
  return true;
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Get WASM module version
 * @returns {string} Version string
 */
function version() {
  ensureInitialized();
  return wasm.version();
}

/**
 * Benchmark DTW performance
 * @param {number} size - Sequence size
 * @param {number} iterations - Number of iterations
 * @returns {number} Average time per iteration (ms)
 */
function benchmarkDtw(size = 100, iterations = 100) {
  ensureInitialized();
  return wasm.benchmark_dtw(size, iterations);
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  init,
  TemporalCompare,
  NanoScheduler,
  StrangeLoop,
  QuicMultistream,
  loadQuicTransport,
  isQuicAvailable,
  isNative,
  version,
  benchmarkDtw
};

// Default export for convenience (mirrors quic.js)
module.exports.default = module.exports;
