/**
 * TypeScript definitions for `midstreamer/quic`.
 *
 * The transport delegates to `agentic-flow/transport/loader`, an
 * optional peer dependency. No native QUIC binding is published, so
 * the loader returns agentic-flow's WebSocket fallback transport.
 */

/**
 * Configuration accepted by `agentic-flow/transport/loader`.
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
 * Load a transport through agentic-flow's loader (currently its
 * WebSocket fallback). Throws a descriptive error if the optional peer
 * dependency `agentic-flow` is not installed.
 */
export function loadQuicTransport(config?: QuicTransportConfig): Promise<AgentTransport>;

/**
 * Whether real QUIC is available, as reported by agentic-flow. False
 * when `agentic-flow` is not installed, and false in every published
 * agentic-flow release to date.
 */
export function isQuicAvailable(): Promise<boolean>;

/**
 * Whether a native QUIC binding backs the loader. Always false.
 */
export function isNative(): boolean;

declare const _default: {
  loadQuicTransport: typeof loadQuicTransport;
  isQuicAvailable: typeof isQuicAvailable;
  isNative: typeof isNative;
};

export default _default;
