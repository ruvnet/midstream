# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the AIMDS project.

## Index

| ADR | Title | Status | Priority |
|-----|-------|--------|----------|
| [ADR-001](ADR-001-runtime-architecture.md) | Runtime Architecture Fix | Proposed | P0 |
| [ADR-002](ADR-002-security-patterns.md) | Security Pattern Improvements | Proposed | P0 |
| [ADR-003](ADR-003-constraint-verification.md) | Constraint Verification Implementation | Proposed | P1 |
| [ADR-004](ADR-004-embedding-system.md) | Embedding System Replacement | Proposed | P1 |
| [ADR-005](ADR-005-mitigation-enforcement.md) | Mitigation Enforcement | Proposed | P0 |

## Context

These ADRs address critical issues identified in the [Deep Technical Review](../../reports/AIDEFENCE_DEEP_REVIEW.md):

1. **Runtime broken** - Package fails to import (ADR-001)
2. **Stub implementations** - Security features return hardcoded values (ADR-003, ADR-005)
3. **Trivial patterns** - Only 15 patterns instead of 50+ claimed (ADR-002)
4. **Mock embeddings** - SHA256 hash instead of semantic embeddings (ADR-004)

## Implementation Priority

**P0 - Critical (must fix for basic functionality)**:
- ADR-001: Runtime must work
- ADR-002: Security patterns must detect threats
- ADR-005: Mitigations must actually enforce

**P1 - High (required for production)**:
- ADR-003: Constraints must verify
- ADR-004: Embeddings must be semantic

## Decision Status

- **Proposed**: Under review
- **Accepted**: Approved for implementation
- **Implemented**: Code complete
- **Superseded**: Replaced by newer ADR
