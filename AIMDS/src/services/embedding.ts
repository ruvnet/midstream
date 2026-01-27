/**
 * Semantic Embedding Service for AIMDS
 *
 * Implements TF-IDF based embeddings with security-aware term weighting.
 * Uses feature hashing to generate fixed-dimension embeddings without
 * external model dependencies.
 *
 * ADR-004: Replaces SHA256-based fake embeddings with semantic understanding
 */

import { createHash } from 'crypto';

/**
 * Security-aware embedding service that provides semantic understanding
 * for prompt injection detection and threat classification.
 */
export class EmbeddingService {
  private readonly dimensions: number;

  /**
   * Pre-computed security term weights for attack detection.
   * Higher weights indicate stronger association with attack patterns.
   */
  private readonly securityTerms: Map<string, number> = new Map([
    // Instruction override patterns
    ['ignore', 0.9],
    ['previous', 0.8],
    ['instructions', 0.9],
    ['forget', 0.85],
    ['disregard', 0.9],
    ['override', 0.85],
    ['bypass', 0.9],
    ['skip', 0.7],

    // System/prompt manipulation
    ['system', 0.85],
    ['prompt', 0.9],
    ['context', 0.6],
    ['hidden', 0.75],
    ['secret', 0.8],
    ['internal', 0.7],

    // Jailbreak terminology
    ['jailbreak', 0.95],
    ['dan', 0.9],
    ['developer', 0.6],
    ['mode', 0.6],
    ['unlimited', 0.75],
    ['unrestricted', 0.85],
    ['unfiltered', 0.85],

    // Role-playing attacks
    ['pretend', 0.8],
    ['roleplay', 0.75],
    ['act', 0.5],
    ['imagine', 0.65],
    ['hypothetical', 0.7],
    ['fictional', 0.65],

    // Privilege escalation
    ['admin', 0.8],
    ['administrator', 0.8],
    ['root', 0.85],
    ['sudo', 0.9],
    ['superuser', 0.85],
    ['elevated', 0.75],
    ['privilege', 0.8],

    // Harmful content indicators
    ['hack', 0.85],
    ['exploit', 0.85],
    ['malware', 0.95],
    ['attack', 0.7],
    ['inject', 0.85],
    ['injection', 0.9],
    ['payload', 0.8],
    ['execute', 0.6],
    ['shell', 0.75],
    ['command', 0.6],

    // Social engineering
    ['trust', 0.5],
    ['believe', 0.4],
    ['actually', 0.5],
    ['really', 0.4],
    ['true', 0.4],
    ['honest', 0.5],

    // Boundary testing
    ['limit', 0.6],
    ['boundary', 0.7],
    ['constraint', 0.65],
    ['restriction', 0.7],
    ['filter', 0.65],
    ['block', 0.6],
    ['censor', 0.75],

    // Encoding/obfuscation
    ['base64', 0.8],
    ['encode', 0.7],
    ['decode', 0.7],
    ['hex', 0.65],
    ['unicode', 0.6],
    ['escape', 0.7],

    // Output manipulation
    ['output', 0.5],
    ['print', 0.4],
    ['display', 0.4],
    ['show', 0.35],
    ['reveal', 0.7],
    ['expose', 0.75],

    // Delimiter attacks
    ['separator', 0.7],
    ['delimiter', 0.75],
    ['boundary', 0.7],
    ['end', 0.4],
    ['begin', 0.4],
    ['start', 0.35],

    // Common attack phrases
    ['you', 0.3],
    ['are', 0.2],
    ['now', 0.4],
    ['new', 0.3],
    ['character', 0.65],
    ['persona', 0.7],
  ]);

  /**
   * N-gram patterns for detecting multi-word attack phrases.
   * Mapped to weight boost factors.
   */
  private readonly attackPhrases: Map<string, number> = new Map([
    ['ignore previous', 1.5],
    ['forget instructions', 1.5],
    ['disregard previous', 1.5],
    ['override system', 1.4],
    ['bypass filter', 1.5],
    ['you are now', 1.3],
    ['pretend you', 1.3],
    ['act as', 1.2],
    ['developer mode', 1.4],
    ['jailbreak mode', 1.6],
    ['ignore all', 1.5],
    ['previous instructions', 1.4],
    ['system prompt', 1.4],
    ['hidden prompt', 1.5],
    ['secret instructions', 1.5],
    ['new instructions', 1.3],
    ['real instructions', 1.4],
    ['true purpose', 1.3],
    ['actual purpose', 1.3],
    ['do anything', 1.2],
    ['no restrictions', 1.4],
    ['no limits', 1.3],
    ['no rules', 1.4],
    ['ignore rules', 1.5],
    ['break rules', 1.4],
    ['forget rules', 1.5],
    ['execute code', 1.3],
    ['run command', 1.3],
    ['shell access', 1.4],
    ['admin access', 1.4],
    ['root access', 1.5],
    ['privilege escalation', 1.5],
    ['give me', 1.1],
    ['grant me', 1.1],
    ['no filters', 1.3],
    ['no limitations', 1.3],
    ['without restrictions', 1.3],
    ['without limits', 1.3],
  ]);

  /**
   * Synonym groups for semantic similarity.
   * Terms in the same group are treated as semantically related.
   */
  private readonly synonymGroups: string[][] = [
    // Access/privilege terms
    ['admin', 'administrator', 'root', 'superuser', 'elevated', 'privilege', 'access', 'grant', 'give'],
    // Ignore/bypass terms
    ['ignore', 'disregard', 'forget', 'bypass', 'skip', 'override'],
    // Instruction terms
    ['instructions', 'rules', 'guidelines', 'constraints', 'restrictions', 'limitations'],
    // Role-play terms
    ['pretend', 'roleplay', 'act', 'imagine', 'hypothetical', 'fictional', 'persona', 'character'],
    // Reveal/expose terms
    ['reveal', 'expose', 'show', 'display', 'print', 'output'],
    // System terms
    ['system', 'prompt', 'context', 'hidden', 'internal', 'secret'],
    // No/without terms
    ['no', 'without', 'unrestricted', 'unfiltered', 'unlimited'],
  ];

  constructor(dimensions: number = 384) {
    this.dimensions = dimensions;
  }

  /**
   * Generate a semantic embedding for the given text.
   * Uses feature hashing with security-aware term weighting.
   *
   * @param text - Input text to embed
   * @returns Normalized embedding vector of specified dimensions
   */
  generateEmbedding(text: string): number[] {
    const normalizedText = text.toLowerCase();
    const tokens = this.tokenize(normalizedText);
    const embedding = new Array(this.dimensions).fill(0);

    // Calculate term frequencies
    const tf = this.calculateTermFrequency(tokens);

    // Apply n-gram boosting for attack phrases
    const phraseBoosts = this.detectAttackPhrases(normalizedText);

    // Generate embedding using feature hashing with security weights
    for (const [term, freq] of tf) {
      const hash = this.hashTerm(term);
      const primaryIdx = hash % this.dimensions;
      const secondaryIdx = (hash >> 8) % this.dimensions;
      const sign = (hash >> 16) % 2 === 0 ? 1 : -1;

      // Get security weight (default 0.1 for unknown terms)
      let secWeight = this.securityTerms.get(term) || 0.1;

      // Apply phrase boost if this term is part of an attack phrase
      const boost = phraseBoosts.get(term) || 1.0;
      secWeight *= boost;

      // TF component with sublinear scaling
      const tfScore = 1 + Math.log(freq);

      // Update primary and secondary positions for better distribution
      embedding[primaryIdx] += sign * tfScore * secWeight;
      embedding[secondaryIdx] += sign * tfScore * secWeight * 0.5;
    }

    // Add synonym group features for semantic similarity
    this.addSynonymGroupFeatures(tokens, embedding);

    // Add character n-gram features for obfuscation detection
    this.addCharacterNgrams(normalizedText, embedding);

    // L2 normalize
    return this.normalize(embedding);
  }

  /**
   * Calculate cosine similarity between two embeddings.
   *
   * @param a - First embedding vector
   * @param b - Second embedding vector
   * @returns Cosine similarity in range [-1, 1]
   */
  cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) {
      throw new Error(`Embedding dimension mismatch: ${a.length} vs ${b.length}`);
    }

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const magnitude = Math.sqrt(normA) * Math.sqrt(normB);
    if (magnitude === 0) return 0;

    return dotProduct / magnitude;
  }

  /**
   * Get the embedding dimensions.
   */
  getDimensions(): number {
    return this.dimensions;
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  /**
   * Tokenize text into words, handling special characters.
   */
  private tokenize(text: string): string[] {
    return text
      .replace(/[^\w\s]/g, ' ')
      .split(/\s+/)
      .filter(t => t.length > 1);
  }

  /**
   * Calculate term frequency for tokens.
   */
  private calculateTermFrequency(tokens: string[]): Map<string, number> {
    const tf = new Map<string, number>();
    for (const token of tokens) {
      tf.set(token, (tf.get(token) || 0) + 1);
    }
    return tf;
  }

  /**
   * Detect attack phrases and return boost factors for component terms.
   */
  private detectAttackPhrases(text: string): Map<string, number> {
    const boosts = new Map<string, number>();

    for (const [phrase, boost] of this.attackPhrases) {
      if (text.includes(phrase)) {
        const words = phrase.split(' ');
        for (const word of words) {
          const currentBoost = boosts.get(word) || 1.0;
          boosts.set(word, Math.max(currentBoost, boost));
        }
      }
    }

    return boosts;
  }

  /**
   * Add features based on synonym group membership.
   * This helps semantically similar terms produce similar embeddings.
   */
  private addSynonymGroupFeatures(tokens: string[], embedding: number[]): void {
    const tokenSet = new Set(tokens);

    for (let groupIdx = 0; groupIdx < this.synonymGroups.length; groupIdx++) {
      const group = this.synonymGroups[groupIdx];
      let matchCount = 0;

      for (const term of group) {
        if (tokenSet.has(term)) {
          matchCount++;
        }
      }

      if (matchCount > 0) {
        // Use group index to determine fixed positions in embedding
        // This ensures texts with synonyms from same group get similar features
        const baseIdx = (groupIdx * 31) % this.dimensions;
        const weight = 0.3 * matchCount;

        // Add features at multiple positions for robustness
        embedding[baseIdx] += weight;
        embedding[(baseIdx + 50) % this.dimensions] += weight * 0.7;
        embedding[(baseIdx + 100) % this.dimensions] += weight * 0.5;
      }
    }
  }

  /**
   * Add character n-gram features for detecting obfuscation patterns.
   * Helps identify leetspeak, unicode tricks, etc.
   */
  private addCharacterNgrams(text: string, embedding: number[]): void {
    const ngrams = new Set<string>();

    // Extract character trigrams
    for (let i = 0; i < text.length - 2; i++) {
      const ngram = text.substring(i, i + 3);
      if (ngram.trim().length >= 2) {
        ngrams.add(ngram);
      }
    }

    // Hash n-grams into embedding with lower weight
    for (const ngram of ngrams) {
      const hash = this.hashTerm(ngram);
      const idx = hash % this.dimensions;
      const sign = (hash >> 8) % 2 === 0 ? 1 : -1;
      embedding[idx] += sign * 0.05; // Small contribution from char n-grams
    }
  }

  /**
   * Hash a term to a 32-bit integer using MD5.
   */
  private hashTerm(term: string): number {
    const hash = createHash('md5').update(term).digest();
    return hash.readUInt32LE(0);
  }

  /**
   * L2 normalize an embedding vector.
   */
  private normalize(embedding: number[]): number[] {
    const norm = Math.sqrt(embedding.reduce((sum, v) => sum + v * v, 0));
    if (norm === 0) {
      // Return zero vector if input is all zeros
      return embedding;
    }
    return embedding.map(v => v / norm);
  }
}

/**
 * Singleton instance for shared use across the application.
 */
let embeddingServiceInstance: EmbeddingService | null = null;

/**
 * Get or create the embedding service singleton.
 *
 * @param dimensions - Embedding dimensions (only used on first call)
 * @returns Shared EmbeddingService instance
 */
export function getEmbeddingService(dimensions: number = 384): EmbeddingService {
  if (!embeddingServiceInstance) {
    embeddingServiceInstance = new EmbeddingService(dimensions);
  }
  return embeddingServiceInstance;
}

/**
 * Reset the embedding service singleton (for testing).
 */
export function resetEmbeddingService(): void {
  embeddingServiceInstance = null;
}
