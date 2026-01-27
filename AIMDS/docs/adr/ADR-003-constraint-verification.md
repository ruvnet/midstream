# ADR-003: Constraint Verification Implementation

**Status**: Proposed
**Date**: 2026-01-27
**Decision Makers**: Engineering Team

## Context

All constraint checking functions currently return `true`:

```typescript
private checkTemporalConstraint(expression: string, action: Action): boolean {
  return true; // Simplified
}
```

This completely bypasses the "formal verification" feature, making security guarantees meaningless.

## Decision

Implement real constraint verification for each type:

### Temporal Constraints
- Time-based access control
- Rate limiting integration
- Session duration enforcement
- Cooldown periods

```typescript
private checkTemporalConstraint(expression: string, action: Action): boolean {
  const constraint = this.parseTemporalExpr(expression);

  switch (constraint.type) {
    case 'within_hours':
      return this.isWithinHours(constraint.start, constraint.end);
    case 'rate_limit':
      return this.checkRateLimit(action.source, constraint.limit, constraint.window);
    case 'cooldown':
      return this.checkCooldown(action.id, constraint.duration);
    default:
      return false; // Deny by default
  }
}
```

### Behavioral Constraints
- Action sequence validation
- Deviation detection
- Pattern compliance

```typescript
private checkBehavioralConstraint(expression: string, action: Action): boolean {
  const pattern = this.parseBehavioralExpr(expression);
  const history = this.getActionHistory(action.source);

  return this.matchesExpectedPattern(history, pattern);
}
```

### Resource Constraints
- Scope validation
- Permission checking
- Quota enforcement

```typescript
private checkResourceConstraint(expression: string, action: Action): boolean {
  const allowed = this.parseResourceExpr(expression);

  return allowed.resources.includes(action.resource) &&
         allowed.methods.includes(action.method) &&
         this.checkQuota(action.source, action.resource);
}
```

### Dependency Constraints
- Prerequisite checking
- State validation
- Transaction ordering

```typescript
private checkDependencyConstraint(expression: string, action: Action): boolean {
  const deps = this.parseDependencyExpr(expression);

  for (const dep of deps) {
    if (!this.isCompleted(dep.action, action.context)) {
      return false;
    }
  }
  return true;
}
```

## Expression Language

Define a DSL for constraints:

```
// Temporal
"within_hours(9,17)"           // Business hours only
"rate_limit(100, 1h)"          // 100 requests per hour
"cooldown(5m)"                 // 5 minute cooldown between actions

// Behavioral
"follows_pattern(auth->request->verify)"
"deviation_threshold(0.3)"

// Resource
"resources(api/v1/*, GET|POST)"
"quota(1000/day)"

// Dependency
"requires(authenticate)"
"after(validate_input)"
```

## Consequences

**Positive**:
- Real security enforcement
- Verifiable constraints
- Audit trail

**Negative**:
- State management required
- Expression parser complexity
- Performance overhead

## Verification

```typescript
// Must fail
assert(!checkTemporalConstraint("within_hours(9,17)", actionAt3AM));
assert(!checkResourceConstraint("resources(api/v1/public/*)", privateApiAction));

// Must pass
assert(checkTemporalConstraint("within_hours(9,17)", actionAt10AM));
assert(checkDependencyConstraint("requires(auth)", authenticatedAction));
```
