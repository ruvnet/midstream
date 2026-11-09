/**
 * A bounded write-behind queue: coalesces writes into one commit.
 *
 * `put` resolves once the entries are queued, not once they are durable.
 * A commit runs when the queue reaches `maxBatch` entries, `flushIntervalMs`
 * after the first queued write, on `flush()`, on `close()`, and on the
 * process's `beforeExit` event. Commits never overlap: each one waits for
 * the one before it.
 *
 * Durability contract: when `await flush()` resolves, every `put` made
 * before the `flush()` call is committed. Without a flush, a crash
 * (SIGKILL, power loss, `process.exit()`, an unhandled signal) loses at
 * most the writes of the last `flushIntervalMs` (plus the commit that was
 * running). `flushIntervalMs: 0` turns batching off: each `put` awaits
 * its own commit.
 *
 * A later `put` for a key already queued replaces the earlier entry, so a
 * commit holds each key once.
 */

export interface WriteBehindOptions {
  /** Commit once this many entries are queued. Default 256. */
  maxBatch?: number;
  /** Commit this long after the first queued write. 0 = write-through. Default 50. */
  flushIntervalMs?: number;
}

export const DEFAULT_MAX_BATCH = 256;
export const DEFAULT_FLUSH_INTERVAL_MS = 50;

type Commit<K, V> = (batch: Map<K, V>) => Promise<void>;

/** Queues with unflushed writes, flushed when the event loop drains. */
const live = new Set<WriteBehind<unknown, unknown>>();
let exitHookInstalled = false;

function installExitHook(): void {
  if (exitHookInstalled) return;
  exitHookInstalled = true;
  // beforeExit fires when the loop empties on a normal exit, and async work
  // is allowed in it. It does not fire on process.exit() or a fatal signal.
  process.on('beforeExit', () => {
    for (const q of live) q.flushOnExit();
  });
}

export class WriteBehind<K, V> {
  readonly maxBatch: number;
  readonly flushIntervalMs: number;
  private pending = new Map<K, V>();
  /** The batch being committed; still readable until the commit settles. */
  private inflight = new Map<K, V>();
  private tail: Promise<void> = Promise.resolve();
  private timer?: NodeJS.Timeout;
  private isClosed = false;
  /** First failure of a commit nobody was awaiting; thrown by the next flush(). */
  private backgroundError?: Error;

  constructor(
    private readonly commit: Commit<K, V>,
    opts: WriteBehindOptions = {},
    private readonly onBackgroundError?: (error: Error) => void
  ) {
    this.maxBatch = Math.max(1, Math.floor(opts.maxBatch ?? DEFAULT_MAX_BATCH));
    this.flushIntervalMs = Math.max(0, opts.flushIntervalMs ?? DEFAULT_FLUSH_INTERVAL_MS);
    live.add(this as WriteBehind<unknown, unknown>);
    installExitHook();
  }

  /** Entries queued and not yet handed to a commit. */
  get size(): number {
    return this.pending.size;
  }

  /** True once `close()` has been called; writes are refused. */
  get closed(): boolean {
    return this.isClosed;
  }

  /** The newest uncommitted entry for `key`, queued or in flight. */
  get(key: K): V | undefined {
    return this.pending.get(key) ?? this.inflight.get(key);
  }

  /** True if a write for `key` is waiting for a later commit than the running one. */
  isQueued(key: K): boolean {
    return this.pending.has(key);
  }

  /** Every uncommitted entry, newest per key. */
  *uncommitted(): IterableIterator<V> {
    for (const [k, v] of this.inflight) if (!this.pending.has(k)) yield v;
    yield* this.pending.values();
  }

  async put(entries: Iterable<[K, V]>): Promise<void> {
    if (this.isClosed) throw new Error('aidefence: store is closed');
    for (const [k, v] of entries) {
      this.pending.delete(k);
      this.pending.set(k, v);
    }
    if (this.pending.size === 0) return;
    if (this.flushIntervalMs === 0) return this.flush();

    if (this.pending.size >= this.maxBatch) {
      const backlog = this.pending.size + this.inflight.size;
      const run = this.startFlush();
      // Bounded: past two batches, the writer waits for the commit.
      if (backlog >= 2 * this.maxBatch) await run;
      else run.catch(e => this.recordBackground(e));
      return;
    }
    if (!this.timer) {
      this.timer = setTimeout(() => {
        this.timer = undefined;
        this.startFlush().catch(e => this.recordBackground(e));
      }, this.flushIntervalMs);
      this.timer.unref();
    }
  }

  /**
   * Commit everything queued so far. Resolves when every write made before
   * this call is durable; rejects if that commit, or an earlier commit
   * nobody awaited, failed.
   */
  async flush(): Promise<void> {
    await this.startFlush();
    const e = this.backgroundError;
    if (e) {
      this.backgroundError = undefined;
      throw e;
    }
  }

  /** Run `fn` after every queued commit, with no commit running beside it. */
  exclusive<T>(fn: () => Promise<T>): Promise<T> {
    const run = this.tail.then(fn);
    this.tail = run.then(
      () => undefined,
      () => undefined
    );
    return run;
  }

  /** Flush, then refuse further writes. */
  async close(): Promise<void> {
    if (this.isClosed) return;
    try {
      await this.flush();
    } finally {
      this.isClosed = true;
      if (this.timer) clearTimeout(this.timer);
      this.timer = undefined;
      live.delete(this as WriteBehind<unknown, unknown>);
    }
  }

  /** Called from the process's beforeExit event. */
  flushOnExit(): void {
    if (this.pending.size > 0) this.startFlush().catch(e => this.recordBackground(e));
  }

  private startFlush(): Promise<void> {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = undefined;
    }
    return this.exclusive(() => this.drain());
  }

  private async drain(): Promise<void> {
    if (this.pending.size === 0) return;
    const batch = this.pending;
    this.pending = new Map();
    this.inflight = batch;
    try {
      await this.commit(batch);
    } finally {
      this.inflight = new Map();
    }
  }

  private recordBackground(e: unknown): void {
    const error = e instanceof Error ? e : new Error(String(e));
    this.backgroundError ??= error;
    this.onBackgroundError?.(error);
  }
}
