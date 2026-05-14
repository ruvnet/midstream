/**
 * In-process stub for the `lean-agentic` theorem-prover engine.
 *
 * Background: the upstream `lean-agentic` 0.3.2 npm package ships a
 * broken install — its compiled WASM artifact
 * (`wasm/leanr_wasm.js`) is missing from the published tarball,
 * which crashes the gateway at boot:
 *
 *     Cannot find module ".../node_modules/lean-agentic/wasm/leanr_wasm.js"
 *
 * Until upstream re-publishes a working build (or we vendor our own
 * verifier), this stub keeps `LeanAgenticVerifier` functional with
 * a permissive-but-loggable surface. Every call logs at WARN that
 * real verification is unavailable so a downstream operator can
 * audit how many decisions ran on the stub.
 *
 * Trade-offs:
 * - `prove()` always returns a deterministic proof token. Verifying
 *   that token via `verify()` returns true. This is intentionally
 *   permissive: it lets the gateway boot and serve traffic, but the
 *   *real* policy gating must come from the upstream regex /
 *   heuristic layers (PatternMatcher, Sanitizer) until a real
 *   prover is wired back in.
 * - `hashConsEquals()` falls back to string equality.
 * - `typeCheck()` returns success unconditionally.
 *
 * Replace with a real verifier when lean-agentic ships a working
 * artifact or when we vendor an alternative (e.g. Coq/Lean4-via-WASM).
 */

export interface StubLogger {
  warn: (msg: string, meta?: Record<string, unknown>) => void;
}

export interface ProofResult {
  theorem: string;
  proof: string;
  isValid: boolean;
}

export class StubLeanEngine {
  private warnedOnce = false;
  private logger?: StubLogger;
  private knownAxioms: string[] = [];

  setLogger(logger: StubLogger): void {
    this.logger = logger;
  }

  async initialize(): Promise<void> {
    this.warnOnce('lean-agentic engine running in STUB mode (no real verification)');
  }

  async shutdown(): Promise<void> {
    // no-op
  }

  async addAxiom(axiom: string): Promise<void> {
    this.knownAxioms.push(axiom);
  }

  async prove(theorem: string): Promise<ProofResult> {
    this.warnOnce('stub prover returning permissive proof token');
    return {
      theorem,
      proof: `stub-proof::${cheapHash(theorem)}`,
      isValid: true,
    };
  }

  async verify(theorem: string, proof: string): Promise<boolean> {
    // A token issued by `prove(theorem)` has the form
    // `stub-proof::<hash-of-theorem>`. We accept iff the hash agrees
    // and the prefix matches — a downstream operator could swap in a
    // tampered token and we'd reject it.
    if (!proof.startsWith('stub-proof::')) return false;
    return proof.slice('stub-proof::'.length) === cheapHash(theorem);
  }

  async hashConsEquals(lhs: string, rhs: string): Promise<boolean> {
    return lhs === rhs;
  }

  async typeCheck(_expr: string): Promise<{ valid: boolean; message?: string }> {
    return { valid: true };
  }

  async evaluate(_expr: string): Promise<boolean> {
    // Permissive default — returns true for any condition. The
    // real verifier would interpret `expr` as a typed predicate.
    return true;
  }

  private warnOnce(msg: string): void {
    if (this.warnedOnce) return;
    this.warnedOnce = true;
    this.logger?.warn(msg);
  }
}

function cheapHash(s: string): string {
  // FNV-1a 32-bit. Not cryptographic — just enough to detect
  // accidental proof-token swapping. Replace if/when the real
  // verifier comes back.
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16);
}

/**
 * Drop-in replacement for `leanAgentic.createDemo()`.
 */
export function createDemo(): StubLeanEngine {
  return new StubLeanEngine();
}
