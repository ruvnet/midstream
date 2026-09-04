/**
 * Pattern pack loader.
 *
 * Packs live in `AIMDS/patterns/<pack>.json` (shared with the Rust core) and
 * follow the schema below. Loading validates every entry and compiles its
 * regex under the declared flags; a pack that fails validation throws.
 */

import { readFileSync, readdirSync, existsSync } from 'fs';
import { join, resolve } from 'path';
import { compactSource } from './normalize';

export type Severity = 'low' | 'medium' | 'high' | 'critical';

export type ThreatKind =
  | 'prompt_injection'
  | 'jailbreak'
  | 'instruction_override'
  | 'role_switching'
  | 'context_manipulation'
  | 'encoding_attack'
  | 'tool_invocation'
  | 'data_exfiltration'
  | 'markup_forgery'
  | 'unknown';

export interface PatternSpec {
  readonly id: string;
  readonly pack: string;
  readonly severity: Severity;
  readonly type: ThreatKind;
  readonly regex: string;
  /** Subset of "imsu". The global flag is rejected (it makes RegExp stateful). */
  readonly flags: string;
  readonly description: string;
  readonly examples: readonly string[];
  readonly confidence?: number;
  /** "portable" (default) compiles in Rust's regex crate; "js" needs lookaround/backrefs. */
  readonly engine?: 'portable' | 'js';
  readonly note?: string;
}

export interface PatternPack {
  readonly pack: string;
  readonly version: string;
  readonly enabledByDefault: boolean;
  readonly description: string;
  readonly patterns: readonly PatternSpec[];
}

export interface CompiledPattern {
  readonly spec: PatternSpec;
  readonly re: RegExp;
  /** Same regex with `\s+`/`\s*` removed, for the whitespace-free text variant. */
  readonly compactRe: RegExp | null;
}

export const SEVERITIES: readonly Severity[] = ['low', 'medium', 'high', 'critical'];
export const THREAT_KINDS: readonly ThreatKind[] = [
  'prompt_injection', 'jailbreak', 'instruction_override', 'role_switching', 'context_manipulation',
  'encoding_attack', 'tool_invocation', 'data_exfiltration', 'markup_forgery', 'unknown',
];

/** Default pattern directory: `<repo>/AIMDS/patterns`, resolved from src/ or dist/. */
export function defaultPatternDir(): string {
  const candidates = [
    resolve(__dirname, '..', '..', 'patterns'),
    resolve(process.cwd(), 'patterns'),
  ];
  for (const c of candidates) if (existsSync(c)) return c;
  return candidates[0];
}

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(`pattern pack: ${msg}`);
}

export function validatePack(raw: unknown, source = '<inline>'): PatternPack {
  assert(raw && typeof raw === 'object', `${source}: not an object`);
  const p = raw as Record<string, unknown>;
  assert(typeof p.pack === 'string' && /^[a-z0-9_]+$/.test(p.pack), `${source}: bad pack name`);
  assert(typeof p.version === 'string', `${source}: missing version`);
  assert(typeof p.enabledByDefault === 'boolean', `${source}: enabledByDefault must be boolean`);
  assert(Array.isArray(p.patterns) && p.patterns.length > 0, `${source}: patterns must be a non-empty array`);
  const ids = new Set<string>();
  for (const entry of p.patterns as unknown[]) {
    assert(entry && typeof entry === 'object', `${source}: pattern entry not an object`);
    const e = entry as Record<string, unknown>;
    assert(typeof e.id === 'string' && e.id.length > 0, `${source}: pattern missing id`);
    assert(!ids.has(e.id), `${source}: duplicate id ${e.id}`);
    ids.add(e.id);
    assert(e.pack === p.pack, `${source}: ${e.id} pack mismatch`);
    assert(SEVERITIES.includes(e.severity as Severity), `${source}: ${e.id} bad severity`);
    assert(THREAT_KINDS.includes(e.type as ThreatKind), `${source}: ${e.id} bad type ${String(e.type)}`);
    assert(typeof e.regex === 'string' && e.regex.length > 0, `${source}: ${e.id} missing regex`);
    assert(typeof e.flags === 'string' && /^[imsu]*$/.test(e.flags), `${source}: ${e.id} flags must be a subset of "imsu"`);
    assert(typeof e.description === 'string', `${source}: ${e.id} missing description`);
    assert(Array.isArray(e.examples), `${source}: ${e.id} examples must be an array`);
    if (e.confidence !== undefined) {
      assert(typeof e.confidence === 'number' && e.confidence > 0 && e.confidence <= 1, `${source}: ${e.id} confidence out of range`);
    }
    if (e.engine !== undefined) assert(e.engine === 'portable' || e.engine === 'js', `${source}: ${e.id} bad engine`);
    try {
      new RegExp(e.regex as string, e.flags as string);
    } catch (err) {
      throw new Error(`pattern pack: ${source}: ${e.id} does not compile: ${(err as Error).message}`);
    }
  }
  return raw as PatternPack;
}

export function loadPackFile(path: string): PatternPack {
  const raw = JSON.parse(readFileSync(path, 'utf8'));
  return validatePack(raw, path);
}

/** Load every `*.json` pack in `dir`, keyed by pack name. */
export function loadPacks(dir = defaultPatternDir()): Map<string, PatternPack> {
  const out = new Map<string, PatternPack>();
  for (const file of readdirSync(dir).filter((f) => f.endsWith('.json')).sort()) {
    const pack = loadPackFile(join(dir, file));
    assert(!out.has(pack.pack), `${file}: duplicate pack ${pack.pack}`);
    out.set(pack.pack, pack);
  }
  return out;
}

export function compilePattern(spec: PatternSpec): CompiledPattern {
  const re = new RegExp(spec.regex, spec.flags);
  let compactRe: RegExp | null = null;
  const compact = compactSource(spec.regex);
  if (compact !== spec.regex) {
    try {
      compactRe = new RegExp(compact, spec.flags);
    } catch {
      compactRe = null;
    }
  }
  return { spec, re, compactRe };
}

export function compilePack(pack: PatternPack): CompiledPattern[] {
  return pack.patterns.map(compilePattern);
}
