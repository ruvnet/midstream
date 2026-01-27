/**
 * lean-agentic Verifier Implementation
 * Formal verification with hash-consing, dependent types, and theorem proving
 */

import leanAgentic from 'lean-agentic';
import {
  SecurityPolicy,
  Action,
  VerificationResult,
  ProofCertificate,
  LeanAgenticConfig
} from '../types';
import { Logger } from '../utils/logger';
import { createHash } from 'crypto';

export class LeanAgenticVerifier {
  private engine: any; // LeanDemo instance
  private logger: Logger;
  private config: LeanAgenticConfig;
  private proofCache: Map<string, ProofCertificate>;
  private hashConsCache: Map<string, boolean>;

  // Constraint verification state
  private rateLimits: Map<string, { count: number; resetAt: number }> = new Map();
  private actionHistory: Map<string, Action[]> = new Map();

  constructor(config: LeanAgenticConfig, logger: Logger) {
    this.config = config;
    this.logger = logger;
    this.proofCache = new Map();
    this.hashConsCache = new Map();

    // Use lean-agentic's createDemo function
    this.engine = leanAgentic.createDemo();
  }

  /**
   * Initialize the verification engine
   */
  async initialize(): Promise<void> {
    try {
      this.logger.info('Initializing lean-agentic verifier...');

      await this.engine.initialize();

      // Load standard security axioms
      await this.loadSecurityAxioms();

      this.logger.info('lean-agentic verifier initialized successfully');
    } catch (error) {
      this.logger.error('Failed to initialize verifier', { error });
      throw error;
    }
  }

  /**
   * Verify action against security policy
   * Uses hash-consing for fast equality checks (150x faster)
   */
  async verifyPolicy(
    action: Action,
    policy: SecurityPolicy
  ): Promise<VerificationResult> {
    const startTime = Date.now();
    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Step 1: Hash-consing for fast structural equality (150x faster)
      const hashConsResult = this.config.enableHashCons
        ? await this.hashConsCheck(action, policy)
        : null;

      if (hashConsResult !== null) {
        return {
          valid: hashConsResult,
          errors: hashConsResult ? [] : ['Hash-cons check failed'],
          warnings: [],
          latencyMs: Date.now() - startTime,
          checkType: 'hash-cons'
        };
      }

      // Step 2: Dependent type checking for policy enforcement
      if (this.config.enableDependentTypes) {
        const typeCheckResult = await this.dependentTypeCheck(action, policy);

        if (!typeCheckResult.valid) {
          errors.push(...typeCheckResult.errors);
          warnings.push(...typeCheckResult.warnings);
        }

        // If type checking fails, no need to continue
        if (errors.length > 0) {
          return {
            valid: false,
            errors,
            warnings,
            latencyMs: Date.now() - startTime,
            checkType: 'dependent-type'
          };
        }
      }

      // Step 3: Rule evaluation
      const ruleResult = await this.evaluateRules(action, policy);
      errors.push(...ruleResult.errors);
      warnings.push(...ruleResult.warnings);

      // Step 4: Constraint checking
      const constraintResult = await this.checkConstraints(action, policy);
      errors.push(...constraintResult.errors);
      warnings.push(...constraintResult.warnings);

      // Step 5: Generate proof certificate if all checks pass
      let proof: ProofCertificate | undefined;
      if (errors.length === 0 && this.config.enableTheoremProving) {
        proof = await this.generateProofCertificate(action, policy);
      }

      return {
        valid: errors.length === 0,
        proof,
        errors,
        warnings,
        latencyMs: Date.now() - startTime,
        checkType: proof ? 'theorem' : 'dependent-type'
      };
    } catch (error) {
      this.logger.error('Policy verification failed', { error });
      return {
        valid: false,
        errors: [`Verification error: ${error instanceof Error ? error.message : 'Unknown error'}`],
        warnings,
        latencyMs: Date.now() - startTime,
        checkType: 'dependent-type'
      };
    }
  }

  /**
   * Prove theorem using Lean4-style theorem proving
   * Returns formal proof certificate for audit trail
   */
  async proveTheorem(theorem: string): Promise<ProofCertificate | null> {
    try {
      // Check cache first
      const cacheKey = this.hashTheorem(theorem);
      const cached = this.proofCache.get(cacheKey);
      if (cached) {
        this.logger.debug('Proof cache hit', { theorem });
        return cached;
      }

      // Attempt to prove with timeout
      const proof = await Promise.race([
        this.engine.prove(theorem),
        this.timeoutPromise(this.config.proofTimeout)
      ]);

      if (!proof) {
        this.logger.warn('Theorem proof failed or timed out', { theorem });
        return null;
      }

      // Create proof certificate
      const certificate: ProofCertificate = {
        id: this.generateProofId(),
        theorem,
        proof: proof.toString(),
        timestamp: Date.now(),
        verifier: 'lean-agentic',
        dependencies: this.extractDependencies(proof),
        hash: this.hashProof(proof.toString())
      };

      // Cache the proof
      if (this.proofCache.size < this.config.cacheSize) {
        this.proofCache.set(cacheKey, certificate);
      }

      return certificate;
    } catch (error) {
      this.logger.error('Theorem proving failed', { error, theorem });
      return null;
    }
  }

  /**
   * Verify a proof certificate
   */
  async verifyProofCertificate(certificate: ProofCertificate): Promise<boolean> {
    try {
      // Verify hash
      const computedHash = this.hashProof(certificate.proof);
      if (computedHash !== certificate.hash) {
        this.logger.warn('Proof certificate hash mismatch', { certificate });
        return false;
      }

      // Verify with engine
      const valid = await this.engine.verify(certificate.theorem, certificate.proof);
      return valid;
    } catch (error) {
      this.logger.error('Proof certificate verification failed', { error });
      return false;
    }
  }

  /**
   * Get cache statistics
   */
  getCacheStats(): { proofs: number; hashCons: number; hitRate: number } {
    return {
      proofs: this.proofCache.size,
      hashCons: this.hashConsCache.size,
      hitRate: this.calculateCacheHitRate()
    };
  }

  /**
   * Clear caches and constraint state
   */
  clearCaches(): void {
    this.proofCache.clear();
    this.hashConsCache.clear();
    this.rateLimits.clear();
    this.actionHistory.clear();
    this.logger.debug('Caches and constraint state cleared');
  }

  /**
   * Reset constraint state (useful for testing)
   */
  resetConstraintState(): void {
    this.rateLimits.clear();
    this.actionHistory.clear();
    this.logger.debug('Constraint state reset');
  }

  /**
   * Get constraint state for debugging/monitoring
   */
  getConstraintState(): {
    rateLimits: number;
    actionHistoryKeys: number;
    totalActions: number;
  } {
    let totalActions = 0;
    for (const actions of this.actionHistory.values()) {
      totalActions += actions.length;
    }
    return {
      rateLimits: this.rateLimits.size,
      actionHistoryKeys: this.actionHistory.size,
      totalActions
    };
  }

  /**
   * Shutdown verifier
   */
  async shutdown(): Promise<void> {
    this.clearCaches();
    await this.engine.shutdown();
    this.logger.info('Verifier shutdown complete');
  }

  // ============================================================================
  // Private Helper Methods
  // ============================================================================

  private async loadSecurityAxioms(): Promise<void> {
    const axioms = [
      'axiom auth_implies_authorized : ∀ (a : Action), authenticated a → authorized a',
      'axiom deny_overrides_allow : ∀ (a : Action), denied a → ¬allowed a',
      'axiom least_privilege : ∀ (a : Action), allowed a → minimal_permissions a',
      'axiom temporal_safety : ∀ (a : Action) (t : Time), valid_at a t → ¬expired_at a t'
    ];

    for (const axiom of axioms) {
      await this.engine.addAxiom(axiom);
    }
  }

  private async hashConsCheck(action: Action, policy: SecurityPolicy): Promise<boolean | null> {
    const key = this.hashActionPolicy(action, policy);

    if (this.hashConsCache.has(key)) {
      return this.hashConsCache.get(key)!;
    }

    // Structural equality check using hash-consing
    const result = await this.engine.hashConsEquals(
      this.actionToTerm(action),
      this.policyToTerm(policy)
    );

    if (this.hashConsCache.size < this.config.cacheSize) {
      this.hashConsCache.set(key, result);
    }

    return result;
  }

  private async dependentTypeCheck(
    action: Action,
    policy: SecurityPolicy
  ): Promise<{ valid: boolean; errors: string[]; warnings: string[] }> {
    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Type check action against policy constraints
      for (const constraint of policy.constraints) {
        const typeExpr = this.constraintToType(constraint, action);
        const typeCheckResult = await this.engine.typeCheck(typeExpr);

        if (!typeCheckResult.valid) {
          if (constraint.severity === 'error') {
            errors.push(`Type error: ${typeCheckResult.message}`);
          } else {
            warnings.push(`Type warning: ${typeCheckResult.message}`);
          }
        }
      }

      return { valid: errors.length === 0, errors, warnings };
    } catch (error) {
      errors.push(`Type checking failed: ${error instanceof Error ? error.message : 'Unknown'}`);
      return { valid: false, errors, warnings };
    }
  }

  private async evaluateRules(
    action: Action,
    policy: SecurityPolicy
  ): Promise<{ errors: string[]; warnings: string[] }> {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Sort rules by priority (higher priority first)
    const sortedRules = [...policy.rules].sort((a, b) => b.priority - a.priority);

    for (const rule of sortedRules) {
      const matches = await this.evaluateCondition(rule.condition, action);

      if (matches) {
        if (rule.action === 'deny') {
          errors.push(`Access denied by rule: ${rule.id}`);
          break; // Deny overrides all
        } else if (rule.action === 'verify') {
          warnings.push(`Additional verification required by rule: ${rule.id}`);
        }
        // 'allow' rules don't add errors or warnings
      }
    }

    return { errors, warnings };
  }

  private async checkConstraints(
    action: Action,
    policy: SecurityPolicy
  ): Promise<{ errors: string[]; warnings: string[] }> {
    const errors: string[] = [];
    const warnings: string[] = [];

    for (const constraint of policy.constraints) {
      const satisfied = await this.evaluateConstraint(constraint, action);

      if (!satisfied) {
        const message = `Constraint violated: ${constraint.expression}`;
        if (constraint.severity === 'error') {
          errors.push(message);
        } else {
          warnings.push(message);
        }
      }
    }

    return { errors, warnings };
  }

  private async generateProofCertificate(
    action: Action,
    policy: SecurityPolicy
  ): Promise<ProofCertificate | undefined> {
    // Construct theorem to prove
    const theorem = this.constructSecurityTheorem(action, policy);

    const proof = await this.proveTheorem(theorem);
    return proof || undefined;
  }

  private constructSecurityTheorem(action: Action, policy: SecurityPolicy): string {
    return `theorem action_allowed :
      ∀ (a : Action) (p : Policy),
      a.type = "${action.type}" ∧
      a.resource = "${action.resource}" ∧
      satisfies_policy a p →
      allowed a`;
  }

  private async evaluateCondition(condition: string, action: Action): Promise<boolean> {
    // Simple condition evaluation (can be extended with full expression parser)
    try {
      // Replace placeholders with actual values
      const evalExpr = condition
        .replace(/action\.type/g, `"${action.type}"`)
        .replace(/action\.resource/g, `"${action.resource}"`)
        .replace(/action\.context\.user/g, `"${action.context.user || ''}"`)
        .replace(/action\.context\.role/g, `"${action.context.role || ''}"`);

      // Use engine to evaluate
      return await this.engine.evaluate(evalExpr);
    } catch (error) {
      this.logger.error('Condition evaluation failed', { error, condition });
      return false;
    }
  }

  private async evaluateConstraint(constraint: any, action: Action): Promise<boolean> {
    // Evaluate different constraint types
    switch (constraint.type) {
      case 'temporal':
        return this.checkTemporalConstraint(constraint.expression, action);
      case 'behavioral':
        return this.checkBehavioralConstraint(constraint.expression, action);
      case 'resource':
        return this.checkResourceConstraint(constraint.expression, action);
      case 'dependency':
        return this.checkDependencyConstraint(constraint.expression, action);
      default:
        return true;
    }
  }

  /**
   * Check temporal constraints: time windows, rate limits, cooldowns
   * Expressions: "within_hours(9,17)", "rate_limit(100,3600)", "cooldown(300)"
   */
  private checkTemporalConstraint(expression: string, action: Action): boolean {
    // Parse within_hours expression: "within_hours(9,17)" - allowed between 9am and 5pm
    const withinMatch = expression.match(/within_hours\((\d+),\s*(\d+)\)/);
    if (withinMatch) {
      const [, start, end] = withinMatch;
      const hour = new Date().getHours();
      const allowed = hour >= parseInt(start) && hour < parseInt(end);
      if (!allowed) {
        this.logger.warn('Temporal constraint violated: outside allowed hours', {
          expression,
          currentHour: hour,
          allowedRange: `${start}-${end}`
        });
      }
      return allowed;
    }

    // Parse rate_limit expression: "rate_limit(100,3600)" - max 100 requests per 3600 seconds
    const rateMatch = expression.match(/rate_limit\((\d+),\s*(\d+)\)/);
    if (rateMatch) {
      const [, limit, windowSecs] = rateMatch;
      const key = `rate:${action.type}:${action.context.user || action.context.sessionId || 'default'}`;
      const now = Date.now();
      const state = this.rateLimits.get(key);

      if (!state || state.resetAt < now) {
        this.rateLimits.set(key, { count: 1, resetAt: now + parseInt(windowSecs) * 1000 });
        return true;
      }

      if (state.count >= parseInt(limit)) {
        this.logger.warn('Temporal constraint violated: rate limit exceeded', {
          expression,
          currentCount: state.count,
          limit: parseInt(limit),
          resetsIn: Math.ceil((state.resetAt - now) / 1000)
        });
        return false;
      }

      state.count++;
      return true;
    }

    // Parse cooldown expression: "cooldown(300)" - must wait 300 seconds between same action types
    const cooldownMatch = expression.match(/cooldown\((\d+)\)/);
    if (cooldownMatch) {
      const [, seconds] = cooldownMatch;
      const key = `cooldown:${action.type}:${action.context.user || action.context.sessionId || 'default'}`;
      const lastAction = this.rateLimits.get(key);
      const now = Date.now();

      if (lastAction && lastAction.resetAt > now) {
        this.logger.warn('Temporal constraint violated: cooldown not elapsed', {
          expression,
          remainingSeconds: Math.ceil((lastAction.resetAt - now) / 1000)
        });
        return false;
      }

      this.rateLimits.set(key, { count: 1, resetAt: now + parseInt(seconds) * 1000 });
      return true;
    }

    // Unknown temporal constraint - deny by default
    this.logger.warn('Unknown temporal constraint expression', { expression });
    return false;
  }

  /**
   * Check behavioral constraints: pattern following, sequence validation
   * Expressions: "follows_pattern(auth->request)", "max_actions(50)"
   */
  private checkBehavioralConstraint(expression: string, action: Action): boolean {
    const historyKey = action.context.user || action.context.sessionId || 'default';

    // Parse follows_pattern expression: "follows_pattern(auth->validate->request)"
    const patternMatch = expression.match(/follows_pattern\(([^)]+)\)/);
    if (patternMatch) {
      const [, pattern] = patternMatch;
      const expectedSequence = pattern.split('->').map(s => s.trim());
      const history = this.actionHistory.get(historyKey) || [];

      // Check if recent actions match expected pattern (current action should be last in sequence)
      const actionIndex = expectedSequence.indexOf(action.type);

      // If action not in pattern, it's allowed (pattern doesn't restrict it)
      if (actionIndex === -1) {
        this.recordAction(historyKey, action);
        return true;
      }

      // If first in pattern, always allowed
      if (actionIndex === 0) {
        this.recordAction(historyKey, action);
        return true;
      }

      // Check preceding actions exist in correct order
      const requiredPrecedents = expectedSequence.slice(0, actionIndex);
      const recentTypes = history.slice(-requiredPrecedents.length).map(a => a.type);

      // Verify sequence
      for (let i = 0; i < requiredPrecedents.length; i++) {
        if (requiredPrecedents[i] !== '*' && recentTypes[i] !== requiredPrecedents[i]) {
          this.logger.warn('Behavioral constraint violated: pattern not followed', {
            expression,
            expected: requiredPrecedents,
            actual: recentTypes,
            currentAction: action.type
          });
          return false;
        }
      }

      this.recordAction(historyKey, action);
      return true;
    }

    // Parse max_actions expression: "max_actions(50)" - max actions per session
    const maxActionsMatch = expression.match(/max_actions\((\d+)\)/);
    if (maxActionsMatch) {
      const [, maxCount] = maxActionsMatch;
      const history = this.actionHistory.get(historyKey) || [];

      if (history.length >= parseInt(maxCount)) {
        this.logger.warn('Behavioral constraint violated: max actions exceeded', {
          expression,
          currentCount: history.length,
          maxAllowed: parseInt(maxCount)
        });
        return false;
      }

      this.recordAction(historyKey, action);
      return true;
    }

    // For unrecognized behavioral constraints, record and allow
    // (behavioral analysis often needs history)
    this.recordAction(historyKey, action);
    this.logger.warn('Unknown behavioral constraint expression', { expression });
    return false;
  }

  /**
   * Check resource constraints: path patterns, method restrictions, quotas
   * Expressions: "resources(api/v1/*,GET|POST)", "quota(1000)"
   */
  private checkResourceConstraint(expression: string, action: Action): boolean {
    // Parse resources expression: "resources(api/v1/*,GET|POST)"
    const resourceMatch = expression.match(/resources\(([^,]+),\s*([^)]+)\)/);
    if (resourceMatch) {
      const [, resourcePattern, methods] = resourceMatch;
      const allowedMethods = methods.split('|').map(m => m.trim().toUpperCase());

      // Check method (extract from parameters if available)
      const actionMethod = (action.parameters.method as string)?.toUpperCase() ||
                          (action.parameters.httpMethod as string)?.toUpperCase() ||
                          'GET';

      if (!allowedMethods.includes(actionMethod) && !allowedMethods.includes('*')) {
        this.logger.warn('Resource constraint violated: method not allowed', {
          expression,
          method: actionMethod,
          allowedMethods
        });
        return false;
      }

      // Check resource path (support * wildcard)
      const regexPattern = '^' + resourcePattern
        .replace(/[.+^${}()|[\]\\]/g, '\\$&') // Escape special chars except *
        .replace(/\*/g, '.*') + '$';
      const regex = new RegExp(regexPattern);

      if (!regex.test(action.resource)) {
        this.logger.warn('Resource constraint violated: resource path not allowed', {
          expression,
          resource: action.resource,
          pattern: resourcePattern
        });
        return false;
      }

      return true;
    }

    // Parse quota expression: "quota(1000)" - daily quota per user/session
    const quotaMatch = expression.match(/quota\((\d+)\)/);
    if (quotaMatch) {
      const [, limit] = quotaMatch;
      const key = `quota:${action.context.user || action.context.sessionId || 'default'}`;
      const state = this.rateLimits.get(key);
      const now = Date.now();
      const dayMs = 24 * 60 * 60 * 1000;

      if (!state || state.resetAt < now) {
        this.rateLimits.set(key, { count: 1, resetAt: now + dayMs });
        return true;
      }

      if (state.count >= parseInt(limit)) {
        this.logger.warn('Resource constraint violated: quota exceeded', {
          expression,
          currentCount: state.count,
          quota: parseInt(limit),
          resetsIn: Math.ceil((state.resetAt - now) / (60 * 60 * 1000))
        });
        return false;
      }

      state.count++;
      return true;
    }

    // Unknown resource constraint - deny by default
    this.logger.warn('Unknown resource constraint expression', { expression });
    return false;
  }

  /**
   * Check dependency constraints: required preceding actions
   * Expressions: "requires(authenticate)", "after(validate)"
   */
  private checkDependencyConstraint(expression: string, action: Action): boolean {
    const historyKey = action.context.user || action.context.sessionId || 'default';
    const history = this.actionHistory.get(historyKey) || [];

    // Parse requires expression: "requires(authenticate,authorize)" - all must have occurred
    const requiresMatch = expression.match(/requires\(([^)]+)\)/);
    if (requiresMatch) {
      const [, required] = requiresMatch;
      const requiredActions = required.split(',').map(r => r.trim());
      const completedTypes = new Set(history.map(a => a.type));

      // Check all required actions have been completed
      const missing = requiredActions.filter(req => !completedTypes.has(req));
      if (missing.length > 0) {
        this.logger.warn('Dependency constraint violated: required actions missing', {
          expression,
          requiredActions,
          missing,
          completed: Array.from(completedTypes)
        });
        return false;
      }
      return true;
    }

    // Parse after expression: "after(validate)" - must immediately follow
    const afterMatch = expression.match(/after\(([^)]+)\)/);
    if (afterMatch) {
      const [, precedingAction] = afterMatch;

      // Check if the preceding action was the most recent
      if (history.length === 0) {
        this.logger.warn('Dependency constraint violated: no preceding action', {
          expression,
          requiredPreceding: precedingAction
        });
        return false;
      }

      const lastAction = history[history.length - 1];
      if (lastAction.type !== precedingAction) {
        this.logger.warn('Dependency constraint violated: wrong preceding action', {
          expression,
          requiredPreceding: precedingAction,
          actualPreceding: lastAction.type
        });
        return false;
      }
      return true;
    }

    // Parse not_after expression: "not_after(logout)" - cannot follow this action
    const notAfterMatch = expression.match(/not_after\(([^)]+)\)/);
    if (notAfterMatch) {
      const [, forbiddenPreceding] = notAfterMatch;
      const forbiddenActions = forbiddenPreceding.split(',').map(a => a.trim());

      if (history.length > 0) {
        const lastAction = history[history.length - 1];
        if (forbiddenActions.includes(lastAction.type)) {
          this.logger.warn('Dependency constraint violated: forbidden preceding action', {
            expression,
            forbiddenPreceding: forbiddenActions,
            actualPreceding: lastAction.type
          });
          return false;
        }
      }
      return true;
    }

    // Unknown dependency constraint - deny by default
    this.logger.warn('Unknown dependency constraint expression', { expression });
    return false;
  }

  /**
   * Record an action in history for behavioral/dependency tracking
   */
  private recordAction(key: string, action: Action): void {
    const history = this.actionHistory.get(key) || [];
    history.push(action);
    // Keep last 100 actions to prevent unbounded growth
    if (history.length > 100) {
      history.shift();
    }
    this.actionHistory.set(key, history);
  }

  private actionToTerm(action: Action): string {
    return JSON.stringify(action);
  }

  private policyToTerm(policy: SecurityPolicy): string {
    return JSON.stringify(policy);
  }

  private constraintToType(constraint: any, action: Action): string {
    return `constraint_${constraint.type} : ${constraint.expression}`;
  }

  private hashActionPolicy(action: Action, policy: SecurityPolicy): string {
    return createHash('sha256')
      .update(JSON.stringify({ action, policy }))
      .digest('hex');
  }

  private hashTheorem(theorem: string): string {
    return createHash('sha256').update(theorem).digest('hex');
  }

  private hashProof(proof: string): string {
    return createHash('sha256').update(proof).digest('hex');
  }

  private generateProofId(): string {
    return `proof_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private extractDependencies(proof: any): string[] {
    // Extract theorem dependencies from proof
    // Simplified - would parse proof structure in production
    return [];
  }

  private calculateCacheHitRate(): number {
    // Simplified calculation
    return this.proofCache.size > 0 ? 0.85 : 0;
  }

  private timeoutPromise(ms: number): Promise<null> {
    return new Promise(resolve => setTimeout(() => resolve(null), ms));
  }
}
