# ADR-001: Runtime Architecture Fix

**Status**: Proposed
**Date**: 2026-01-27
**Decision Makers**: Engineering Team

## Context

The AIMDS package is broken at runtime due to:
1. Missing module dependencies when importing dist/
2. TypeScript build produces artifacts that can't be executed
3. Dependencies (express, agentdb, lean-agentic) not resolved at runtime

Current error:
```
Error: Cannot find module 'express'
```

## Decision

Implement a self-contained build architecture:

### Option A: Bundle Dependencies (Recommended)
- Use esbuild/rollup to bundle all dependencies
- Create single-file distribution
- Include type definitions
- Zero external runtime dependencies

### Option B: Peer Dependencies
- Mark critical deps as peerDependencies
- Document required installations
- Provide installation script

### Option C: Native ESM
- Convert to native ES modules
- Use dynamic imports where needed
- Better tree-shaking

## Chosen: Option A (Bundle Dependencies)

**Rationale**:
- Zero-friction installation
- No dependency conflicts
- Predictable runtime behavior
- Smaller attack surface (vendored deps)

## Implementation

1. Add esbuild configuration
2. Create `npm run bundle` script
3. Generate `dist/aimds.bundle.js`
4. Include sourcemaps for debugging
5. Generate `.d.ts` declarations

## Consequences

**Positive**:
- Single-file distribution works anywhere
- No runtime dependency resolution
- Faster startup time

**Negative**:
- Larger bundle size (~2MB estimated)
- Updates require rebuild
- Duplicate deps if consumer also uses them

## Verification

```bash
# Must work after implementation
node -e "require('./dist/aimds.bundle.js')"
npm test
```
