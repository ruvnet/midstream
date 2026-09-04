/**
 * Pack-driven prompt-injection detection.
 *
 * Pattern data lives in `AIMDS/patterns/*.json` and is shared with the Rust
 * core; this module only loads, compiles, normalises, decodes, and matches.
 */

export {
  InjectionDetector,
  createInjectionDetector,
  getInjectionDetector,
} from './engine';
export type { InjectionDetectorConfig, InjectionReport, InjectionThreat, PackName } from './engine';
export {
  loadPacks,
  loadPackFile,
  validatePack,
  compilePack,
  compilePattern,
  defaultPatternDir,
  SEVERITIES,
  THREAT_KINDS,
} from './pattern-loader';
export type { PatternPack, PatternSpec, CompiledPattern, Severity, ThreatKind } from './pattern-loader';
export { normalizeText, textVariants, compactSource } from './normalize';
export type { TextVariant } from './normalize';
export { extractDecodedCandidates, rot13, reverseText, printableRatio } from './decoders';
export type { DecodedCandidate, DecoderOptions, Encoding } from './decoders';
