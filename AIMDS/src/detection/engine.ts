/**
 * InjectionDetector: pack-driven prompt-injection detector.
 *
 * Pipeline: normalise -> text variants -> one regex exec per (pattern,
 * variant) -> bounded decode of encoded blobs -> one re-scan of each decoded
 * string with every pack except encoded_instruction (no recursion).
 */

import {
  CompiledPattern,
  PatternPack,
  Severity,
  ThreatKind,
  compilePack,
  loadPacks,
} from './pattern-loader';
import { normalizeText, textVariants, TextVariant } from './normalize';
import { DecoderOptions, Encoding, extractDecodedCandidates } from './decoders';

export type PackName =
  | 'core'
  | 'tool_invocation'
  | 'exfil_url'
  | 'encoded_instruction'
  | 'slack_markup_forgery'
  | 'instruction_override_i18n'
  | (string & {});

export interface InjectionDetectorConfig {
  /** Per-pack enable flags. Unlisted packs use their `enabledByDefault`. */
  readonly packs?: Partial<Record<PackName, boolean>>;
  /** Directory of pack JSON files. Defaults to `<repo>/AIMDS/patterns`. */
  readonly patternDir?: string;
  /** Preloaded packs (tests). Overrides `patternDir`. */
  readonly preloaded?: readonly PatternPack[];
  /** Scan obfuscation variants (separators, leet, compact). Default true. */
  readonly variants?: boolean;
  /** Decode base64/hex/url blobs and re-scan once. Default true when encoded_instruction is enabled. */
  readonly decode?: boolean;
  readonly decoder?: DecoderOptions;
  /** Inputs longer than this are truncated before scanning. Default 262144. */
  readonly maxInputChars?: number;
}

export interface InjectionThreat {
  readonly id: string;
  readonly pack: string;
  readonly type: ThreatKind;
  readonly severity: Severity;
  readonly confidence: number;
  readonly description: string;
  readonly match: string;
  readonly offset: number;
  /** Which text variant matched: base, separators, leet, compact. */
  readonly variant: TextVariant['name'];
  /** Set when the match was found in decoded content. */
  readonly decodedFrom?: Encoding;
}

export interface InjectionReport {
  readonly safe: boolean;
  readonly maxSeverity: Severity | null;
  readonly threats: readonly InjectionThreat[];
  readonly scanned: { variants: number; decoded: number; patterns: number };
  readonly truncated: boolean;
  readonly durationMs: number;
}

const SEVERITY_RANK: Record<Severity, number> = { low: 0, medium: 1, high: 2, critical: 3 };
const VARIANT_PENALTY: Record<TextVariant['name'], number> = { base: 0, separators: 0.05, leet: 0.1, compact: 0.15 };

export class InjectionDetector {
  private readonly packs: Map<string, PatternPack>;
  private readonly enabled: Set<string>;
  private readonly compiled: CompiledPattern[];
  private readonly rescan: CompiledPattern[];
  private readonly cfg: Required<Pick<InjectionDetectorConfig, 'variants' | 'decode' | 'maxInputChars'>> & { decoder: DecoderOptions };

  constructor(config: InjectionDetectorConfig = {}) {
    this.packs = config.preloaded
      ? new Map(config.preloaded.map((p) => [p.pack, p]))
      : loadPacks(config.patternDir);
    this.enabled = new Set<string>();
    for (const [name, pack] of this.packs) {
      const flag = config.packs?.[name];
      if (flag === true || (flag === undefined && pack.enabledByDefault)) this.enabled.add(name);
    }
    this.compiled = [];
    for (const name of this.enabled) this.compiled.push(...compilePack(this.packs.get(name)!));
    this.rescan = this.compiled.filter((c) => c.spec.pack !== 'encoded_instruction');
    this.cfg = {
      variants: config.variants ?? true,
      decode: config.decode ?? this.enabled.has('encoded_instruction'),
      maxInputChars: config.maxInputChars ?? 262144,
      decoder: config.decoder ?? {},
    };
  }

  /** Names of packs that are loaded and enabled. */
  enabledPacks(): string[] {
    return [...this.enabled];
  }

  /** Total compiled patterns across enabled packs. */
  patternCount(): number {
    return this.compiled.length;
  }

  detect(input: string): InjectionReport {
    const start = performance.now();
    const truncated = input.length > this.cfg.maxInputChars;
    const text = truncated ? input.slice(0, this.cfg.maxInputChars) : input;
    const normalized = normalizeText(text);
    const variants: TextVariant[] = this.cfg.variants
      ? textVariants(normalized)
      : [{ name: 'base', text: normalized, compactForm: false }];

    const found: InjectionThreat[] = [];
    for (const v of variants) this.scanVariant(v, this.compiled, found, undefined);

    let decoded = 0;
    if (this.cfg.decode) {
      for (const cand of extractDecodedCandidates(normalized, this.cfg.decoder)) {
        decoded++;
        const inner = normalizeText(cand.decoded);
        const innerVariants: TextVariant[] = this.cfg.variants
          ? textVariants(inner, 8192)
          : [{ name: 'base', text: inner, compactForm: false }];
        for (const v of innerVariants) this.scanVariant(v, this.rescan, found, cand.encoding, cand.offset);
      }
    }

    const threats = dedupe(found);
    const maxSeverity = threats.length ? threats[0].severity : null;
    return {
      safe: threats.length === 0,
      maxSeverity,
      threats,
      scanned: { variants: variants.length, decoded, patterns: this.compiled.length },
      truncated,
      durationMs: performance.now() - start,
    };
  }

  private scanVariant(
    v: TextVariant,
    patterns: readonly CompiledPattern[],
    out: InjectionThreat[],
    decodedFrom: Encoding | undefined,
    baseOffset = 0,
  ): void {
    for (const c of patterns) {
      const re = v.compactForm ? c.compactRe : c.re;
      if (!re) continue;
      const m = re.exec(v.text);
      if (!m || m[0].length === 0) continue;
      const base = c.spec.confidence ?? 0.7;
      const confidence = Math.max(0.1, Math.round((base - VARIANT_PENALTY[v.name] - (decodedFrom ? 0.05 : 0)) * 100) / 100);
      out.push({
        id: c.spec.id,
        pack: c.spec.pack,
        type: c.spec.type,
        severity: c.spec.severity,
        confidence,
        description: c.spec.description,
        match: m[0].length > 160 ? `${m[0].slice(0, 157)}...` : m[0],
        offset: baseOffset + m.index,
        variant: v.name,
        ...(decodedFrom ? { decodedFrom } : {}),
      });
    }
  }
}

/** One threat per pattern id (highest confidence), sorted by severity then confidence. */
function dedupe(threats: InjectionThreat[]): InjectionThreat[] {
  const best = new Map<string, InjectionThreat>();
  for (const t of threats) {
    const prev = best.get(t.id);
    if (!prev || t.confidence > prev.confidence) best.set(t.id, t);
  }
  return [...best.values()].sort(
    (a, b) => SEVERITY_RANK[b.severity] - SEVERITY_RANK[a.severity] || b.confidence - a.confidence,
  );
}

let defaultDetector: InjectionDetector | null = null;

/** Shared detector with every default-on pack. */
export function getInjectionDetector(): InjectionDetector {
  if (!defaultDetector) defaultDetector = new InjectionDetector();
  return defaultDetector;
}

export function createInjectionDetector(config?: InjectionDetectorConfig): InjectionDetector {
  return new InjectionDetector(config);
}
