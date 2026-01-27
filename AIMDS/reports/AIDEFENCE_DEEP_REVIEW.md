# aidefence/AIMDS Deep Technical Review

**Date**: 2026-01-27
**Reviewer**: Claude Code
**Status**: **CRITICAL - NOT PRODUCTION VIABLE**

---

## Executive Summary

This deep review validates and expands upon the initial assessment. The aidefence/AIMDS package presents critical issues that make it unsuitable for production security applications:

| Issue | Severity | Validated |
|-------|----------|-----------|
| Runtime broken (missing modules) | CRITICAL | YES |
| 10 versions in 1 day (lean-agentic) | HIGH | YES |
| Core features are stubs/mocks | CRITICAL | YES |
| Trivial regex patterns (15 not 50+) | HIGH | YES |
| Supply chain risk (single-author chain) | HIGH | YES |

---

## 1. RUNTIME VIABILITY: CRITICAL FAILURE

### Runtime Test Result

```bash
$ node -e "require('./dist/index.js')"
Error: Cannot find module 'express'
```

The dist/ directory exists but fails at runtime because dependencies aren't bundled and the package doesn't install cleanly.

**Evidence**: Running `require('./dist/index.js')` produces `MODULE_NOT_FOUND` errors.

### Build Dependency Issues

The TypeScript source has unresolvable imports:
- `src/agentdb/client.ts:6` - requires `agentdb`
- `src/gateway/server.ts:6` - requires `express`
- `src/lean-agentic/verifier.ts:6` - requires `lean-agentic`

---

## 2. VERSION HISTORY: ARTIFICIAL/RUSHED RELEASES

### lean-agentic npm Package

**All 10 versions published in ONE DAY (2025-10-25)**:

```
0.1.0: 2025-10-25T15:43:04.972Z
0.1.1: 2025-10-25T15:44:04.448Z  (+1 minute)
0.1.2: 2025-10-25T15:53:09.170Z  (+9 minutes)
0.2.0: 2025-10-25T16:06:51.567Z  (+13 minutes)
0.2.1: 2025-10-25T16:15:16.246Z  (+8 minutes)
0.2.2: 2025-10-25T16:24:49.562Z  (+9 minutes)
0.2.3: 2025-10-25T16:25:40.653Z  (+51 seconds)
0.3.0: 2025-10-25T17:07:57.567Z  (+42 minutes)
0.3.1: 2025-10-25T17:20:46.470Z  (+12 minutes)
0.3.2: 2025-10-25T17:26:31.230Z  (+6 minutes)
```

**Total development time**: ~1 hour 43 minutes for 10 versions claiming "formal verification", "150x faster hash-consing", and "Byzantine consensus".

### agentdb npm Package

Over 90 versions published, with rapid version inflation from `1.x` to `2.0.0-alpha.3.x`.

---

## 3. CORE FEATURES ARE STUBS/MOCKS

### 3.1 Constraint Checking - ALL RETURN TRUE

**File**: `src/lean-agentic/verifier.ts:406-424`

```typescript
private checkTemporalConstraint(expression: string, action: Action): boolean {
  return true; // Simplified
}

private checkBehavioralConstraint(expression: string, action: Action): boolean {
  return true; // Simplified
}

private checkResourceConstraint(expression: string, action: Action): boolean {
  return true; // Simplified
}

private checkDependencyConstraint(expression: string, action: Action): boolean {
  return true; // Simplified
}
```

**Impact**: The "formal verification" feature does nothing - all constraints pass automatically.

### 3.2 Embeddings - SHA256 Hash Instead of ML Model

**File**: `src/gateway/server.ts:412-430`

```typescript
// Hash-based embedding for demo (use BERT/etc in production)
const hash = createHash('sha256').update(text).digest();
const embedding = new Array(384);

for (let i = 0; i < 384; i++) {
  embedding[i] = hash[i % hash.length] / 255;
}
```

**Impact**: Vector similarity search is meaningless when embeddings are just repeated hash bytes. The claimed "150x faster" search is comparing non-semantic data.

### 3.3 Mitigations - Log Only, No Actual Enforcement

**File**: `crates/aimds-response/src/mitigations.rs:69-147`

Every mitigation action logs and increments a counter:

```rust
async fn execute_block(&self, context: &ThreatContext, reason: &str) -> Result<String> {
    tracing::info!("Blocking request from {} - Reason: {}", context.source_id, reason);
    // In production, this would integrate with firewall/WAF
    metrics::counter!("mitigation.blocks").increment(1);
    Ok(action_id)
}

async fn execute_rate_limit(&self, context: &ThreatContext, duration: Duration) -> Result<String> {
    tracing::info!("Rate limiting {} for {:?}", context.source_id, duration);
    // In production, integrate with rate limiter (Redis, etc.)
    metrics::counter!("mitigation.rate_limits").increment(1);
    Ok(action_id)
}
```

**Impact**: No actual blocking, rate limiting, or verification occurs. Comments explicitly state "In production, this would..." but production never came.

---

## 4. SECURITY PATTERNS: TRIVIAL COVERAGE

### Claimed: "50+ prompt injection patterns"
### Actual: 15 patterns

**File**: `crates/aimds-detection/src/pattern_matcher.rs:167-191`

**Aho-Corasick patterns (10)**:
```rust
vec![
    "ignore previous instructions",
    "disregard all prior",
    "forget everything",
    "system prompt",
    "admin mode",
    "developer mode",
    "jailbreak",
    "unrestricted mode",
    "bypass filter",
    "override safety",
]
```

**Regex patterns (5)**:
```rust
vec![
    r"(?i)ignore\s+(all|previous|prior)\s+instructions",
    r"(?i)system\s*:\s*you\s+are",
    r"(?i)act\s+as\s+(an?\s+)?unrestricted",
    r"(?i)pretend\s+you\s+are\s+(not\s+)?bound",
    r"(?i)disregard\s+your\s+(programming|rules)",
]
```

### Bypass Examples

These trivial patterns are easily bypassed:

```
"Please disregard the above directives"  // "instructions" → "directives"
"Forget the prior guidelines"            // "instructions" → "guidelines"
"System: you shall be..."                // "are" → "shall be"
"Act as if you were unrestricted"        // "as an" → "as if you were"
```

---

## 5. SUPPLY CHAIN RISK

### Single-Author Ecosystem

| Package | Author | Purpose |
|---------|--------|---------|
| aimds-gateway | ruv.io | This package |
| agentdb | ruv.io | Vector database dependency |
| lean-agentic | ruv.io | Theorem prover dependency |

All three packages in the dependency chain are by the same author with no independent validation or security audits.

### No Independent Verification

- No external security audits
- No independent maintainers
- No usage in production systems
- Self-published "security audit" rates it 45/100

---

## 6. SELF-DOCUMENTED ISSUES

The project's own security audit (`reports/SECURITY_AUDIT_REPORT.md`) documents:

| Category | Score |
|----------|-------|
| **Overall Security** | 45/100 (CRITICAL) |
| Secrets Management | 0/100 |
| Authentication | 20/100 |
| Transport Security | 0/100 |

**Critical issues self-documented**:
- Hardcoded API keys in `.env`
- No HTTPS/TLS support
- No API authentication
- CORS allows all origins
- Compilation errors prevent deployment

---

## 7. RECOMMENDED ALTERNATIVES

For actual production security, use these proven alternatives:

| Feature | aidefence Claim | Recommended Alternative |
|---------|-----------------|-------------------------|
| Rate limiting | Stub implementation | **express-rate-limit** (16M+ weekly downloads) |
| Input validation | Returns true | **zod** (10M+ weekly downloads) |
| Security headers | None | **helmet** (2M+ weekly downloads) |
| Logging | Basic winston | **winston** + **pino** (production proven) |
| Prompt injection | 15 trivial patterns | **promptfoo** (red-teaming), **rebuff** |
| Vector search | SHA256 mock | **pgvector**, **Pinecone**, **Weaviate** |

### Production-Ready AI Security Stack

```javascript
// Instead of aidefence, use:
const helmet = require('helmet');           // Security headers
const rateLimit = require('express-rate-limit');  // Rate limiting
const { z } = require('zod');              // Input validation
const winston = require('winston');        // Structured logging

// For AI-specific security:
// - promptfoo for prompt injection testing
// - rebuff for runtime detection
// - guardrails-ai for output validation
```

---

## 8. EVIDENCE SUMMARY

| Issue | File | Line(s) |
|-------|------|---------|
| Stub constraints (return true) | `src/lean-agentic/verifier.ts` | 406-424 |
| Mock embeddings (SHA256) | `src/gateway/server.ts` | 412-430 |
| Stub mitigations (log only) | `crates/aimds-response/src/mitigations.rs` | 69-147 |
| Only 15 patterns | `crates/aimds-detection/src/pattern_matcher.rs` | 167-191 |
| Self-rated 45/100 | `reports/SECURITY_AUDIT_REPORT.md` | 14 |
| 10 versions in 1 day | npm registry (lean-agentic) | - |
| Runtime failure | `dist/index.js` | - |

---

## Conclusion

**DO NOT USE aidefence/AIMDS for any security-sensitive application.**

The package is:
1. **Broken at runtime** - Cannot be imported without errors
2. **Functionally hollow** - Security features are stubs returning hardcoded values
3. **Misleading documentation** - Claims features that don't exist
4. **Unmaintained quality** - 10 versions published in under 2 hours
5. **Supply chain risky** - Single-author dependency chain with no audits

For production AI security, use proven open-source tools (helmet, zod, express-rate-limit) combined with specialized AI security frameworks (promptfoo, rebuff, guardrails-ai).

---

*Review conducted with full source code access and runtime testing.*
