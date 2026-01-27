# ADR-002: Security Pattern Implementation

**Status**: Proposed
**Date**: 2026-01-27
**Decision Makers**: Security Team

## Context

Current implementation has only 15 trivial patterns (10 string + 5 regex) that are easily bypassed:
- Simple string matching: "ignore previous instructions"
- Basic regex: `(?i)ignore\s+(all|previous|prior)\s+instructions`

Advertised as "50+ patterns" but actual coverage is minimal.

## Decision

Implement comprehensive multi-layer detection:

### Layer 1: Enhanced Pattern Matching (100+ patterns)
- Prompt injection patterns (30+)
- Jailbreak variations (20+)
- System prompt extraction (15+)
- Role manipulation (15+)
- Output manipulation (10+)
- Encoding bypass attempts (10+)

### Layer 2: Semantic Analysis
- Sentence transformer embeddings (real ML, not SHA256)
- Cosine similarity against known attack vectors
- Dynamic threshold based on context

### Layer 3: Structural Analysis
- Token sequence analysis
- Unusual formatting detection
- Multi-language attack detection
- Unicode/encoding abuse

### Layer 4: Behavioral Heuristics
- Request frequency anomalies
- Session pattern deviation
- Context window abuse

## Implementation Priority

```
P0 (Critical):
- Expand to 100+ patterns minimum
- Add encoding-aware detection (base64, unicode)
- Add multi-language patterns

P1 (High):
- Integrate sentence-transformers
- Add structural analysis
- Implement confidence scoring

P2 (Medium):
- Behavioral heuristics
- Pattern learning from incidents
- Real-time pattern updates
```

## Pattern Categories to Add

```rust
// Encoding bypass
vec![
    r"(?i)base64\s*decode",
    r"(?i)eval\s*\(",
    r"\\x[0-9a-f]{2}",
    r"&#x?[0-9a-f]+;",
]

// Roleplay manipulation
vec![
    r"(?i)you\s+are\s+(now|actually)",
    r"(?i)from\s+now\s+on\s+you",
    r"(?i)let('s|us)\s+play\s+a\s+game",
    r"(?i)pretend\s+(to\s+be|you're)",
]

// System extraction
vec![
    r"(?i)what\s+(is|are)\s+your\s+(rules|instructions)",
    r"(?i)show\s+me\s+your\s+system\s+prompt",
    r"(?i)repeat\s+(everything|all)\s+(above|before)",
]
```

## Consequences

**Positive**:
- Real security coverage
- Multi-layer defense
- Measurable detection rates

**Negative**:
- Higher compute per request
- Pattern maintenance burden
- Potential false positives

## Verification

```bash
# Pattern coverage tests
cargo test pattern_coverage -- --nocapture
# False positive rate < 1%
cargo test false_positive_rate
# Detection rate > 95% on test corpus
cargo test detection_benchmark
```
