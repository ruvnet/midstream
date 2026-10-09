/**
 * TypeScript definitions for `midstreamer/quic`.
 *
 * The QUIC transport delegates to `agentic-flow/transport/loader`,
 * which is an optional peer dependency — install `agentic-flow` to
 * actually load a transport; `isQuicAvailable()` probes without it.
 */

/**
 * Configuration accepted by the underlying
 * `agentic-flow/transport/loader` QUIC stack.
 */
export interface QuicTransportConfig {
  serverName?: string;
  maxIdleTimeoutMs?: number;
  maxConcurrentStreams?: number;
  enable0Rtt?: boolean;
  tls?: unknown;
}

/**
 * Transport surface returned by `loadQuicTransport`, matching the
 * `agentic-flow/transport/loader` AgentTransport contract.
 */
export interface AgentTransport {
  send(...args: unknown[]): Promise<unknown>;
  receive(...args: unknown[]): Promise<unknown>;
  request(...args: unknown[]): Promise<unknown>;
  sendBatch(...args: unknown[]): Promise<unknown>;
  getStats(...args: unknown[]): unknown;
  close(): Promise<void>;
}

/**
 * Load a real QUIC transport (UDP sockets, TLS, full handshake).
 * Throws with a descriptive error if the optional peer dependency
 * `agentic-flow` is not installed.
 */
export function loadQuicTransport(config?: QuicTransportConfig): Promise<AgentTransport>;

/**
 * Probe whether QUIC is available without instantiating a transport.
 * Resolves false when `agentic-flow` is not installed or its loader
 * reports the stack is not ready.
 */
export function isQuicAvailable(): Promise<boolean>;

/**
 * Reports that this loader is backed by the real agentic-flow QUIC
 * stack rather than a stub.
 */
export function isNative(): boolean;

declare const _default: {
  loadQuicTransport: typeof loadQuicTransport;
  isQuicAvailable: typeof isQuicAvailable;
  isNative: typeof isNative;
};

export default _default;
