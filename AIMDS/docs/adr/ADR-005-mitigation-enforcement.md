# ADR-005: Mitigation Enforcement Implementation

**Status**: Proposed
**Date**: 2026-01-27
**Decision Makers**: Security/Platform Team

## Context

All mitigation actions currently only log and increment counters:

```rust
async fn execute_block(&self, context: &ThreatContext, reason: &str) -> Result<String> {
    tracing::info!("Blocking request...");
    // In production, this would integrate with firewall/WAF
    metrics::counter!("mitigation.blocks").increment(1);
    Ok(action_id)
}
```

No actual enforcement occurs - threats are logged but not stopped.

## Decision

Implement real enforcement mechanisms for each mitigation type:

### Block Action
- Integration with reverse proxy (nginx, envoy)
- IP blocklist management
- Request rejection with proper status codes

```rust
async fn execute_block(&self, context: &ThreatContext, reason: &str) -> Result<String> {
    let action_id = Uuid::new_v4().to_string();

    // 1. Add to blocklist
    self.blocklist.add(
        &context.source_id,
        BlocklistEntry {
            reason: reason.to_string(),
            expires: Utc::now() + Duration::hours(1),
            severity: context.threat_level,
        }
    ).await?;

    // 2. Signal reverse proxy (if configured)
    if let Some(proxy) = &self.proxy_client {
        proxy.update_blocklist(&context.source_id).await?;
    }

    // 3. Terminate active connections
    self.connection_manager.terminate(&context.source_id).await?;

    // 4. Record for audit
    self.audit_log.record_block(&action_id, context, reason).await?;

    metrics::counter!("mitigation.blocks.enforced").increment(1);
    Ok(action_id)
}
```

### Rate Limit Action
- Redis-based rate limiting
- Sliding window algorithm
- Per-source quotas

```rust
async fn execute_rate_limit(&self, context: &ThreatContext, duration: Duration) -> Result<String> {
    let action_id = Uuid::new_v4().to_string();

    // Apply rate limit via Redis
    let key = format!("ratelimit:{}", context.source_id);
    self.redis.set_ex(&key, "1", duration.as_secs()).await?;

    // Configure rate (requests per window)
    let rate_config = RateLimitConfig {
        requests: 10,
        window: duration,
        penalty: PenaltyAction::Delay(Duration::from_secs(5)),
    };

    self.rate_limiter.apply(&context.source_id, rate_config).await?;

    metrics::counter!("mitigation.rate_limits.enforced").increment(1);
    Ok(action_id)
}
```

### Verification Challenge
- CAPTCHA integration
- Challenge-response tokens
- Proof-of-work requirements

```rust
async fn execute_verification(&self, context: &ThreatContext, challenge: &ChallengeType) -> Result<String> {
    let action_id = Uuid::new_v4().to_string();

    let challenge_token = match challenge {
        ChallengeType::Captcha => {
            self.captcha_service.generate_challenge(&context.source_id).await?
        },
        ChallengeType::Token => {
            self.token_service.issue_challenge(&context.source_id).await?
        },
        ChallengeType::ProofOfWork => {
            self.pow_service.generate_puzzle(&context.source_id, Difficulty::Medium).await?
        },
    };

    // Mark session as requiring verification
    self.session_store.require_challenge(
        &context.session_id,
        challenge_token
    ).await?;

    metrics::counter!("mitigation.verifications.enforced").increment(1);
    Ok(action_id)
}
```

### Alert Action
- PagerDuty/Opsgenie integration
- Slack/Teams webhooks
- Email escalation

```rust
async fn execute_alert(&self, context: &ThreatContext, priority: &AlertPriority) -> Result<String> {
    let action_id = Uuid::new_v4().to_string();

    let alert = Alert {
        id: action_id.clone(),
        title: format!("Security threat detected: {}", context.threat_id),
        description: context.to_description(),
        priority: priority.clone(),
        source: context.source_id.clone(),
        timestamp: Utc::now(),
    };

    // Send to configured channels
    let futures: Vec<_> = self.alert_channels.iter()
        .map(|channel| channel.send(&alert))
        .collect();

    join_all(futures).await;

    metrics::counter!("mitigation.alerts.enforced").increment(1);
    Ok(action_id)
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    AIMDS Gateway                            │
├─────────────────────────────────────────────────────────────┤
│  Detection  →  Analysis  →  Mitigation Engine               │
│                              │                              │
│              ┌───────────────┼───────────────┐              │
│              ▼               ▼               ▼              │
│         ┌────────┐    ┌──────────┐    ┌───────────┐        │
│         │Blocklist│    │Rate Limit│    │ Challenge │        │
│         │ Store   │    │  (Redis) │    │  Service  │        │
│         └────────┘    └──────────┘    └───────────┘        │
│              │               │               │              │
└──────────────┼───────────────┼───────────────┼──────────────┘
               │               │               │
               ▼               ▼               ▼
         ┌──────────┐   ┌───────────┐   ┌──────────┐
         │  Nginx/  │   │   Redis   │   │ CAPTCHA  │
         │  Envoy   │   │  Cluster  │   │  API     │
         └──────────┘   └───────────┘   └──────────┘
```

## Consequences

**Positive**:
- Real threat mitigation
- Measurable security outcomes
- Automated response

**Negative**:
- External dependencies (Redis, proxy)
- Configuration complexity
- Potential for false positives blocking legitimate users

## Verification

```rust
#[tokio::test]
async fn test_block_actually_blocks() {
    let engine = MitigationEngine::new(test_config()).await;
    let context = ThreatContext::test_context();

    // Execute block
    engine.execute_block(&context, "test").await.unwrap();

    // Verify blocklist contains source
    assert!(engine.blocklist.is_blocked(&context.source_id).await);

    // Verify subsequent requests are rejected
    let result = engine.check_allowed(&context.source_id).await;
    assert!(!result.allowed);
}
```
