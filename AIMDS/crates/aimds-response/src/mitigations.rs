//! Mitigation actions and execution framework with real enforcement
//!
//! This module provides actual blocking and rate limiting enforcement,
//! not just logging stubs. All mitigations are tracked in thread-safe
//! data structures with automatic expiration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::adaptive::{AlertPriority, ChallengeType};
use crate::meta_learning::ThreatIncident;
use crate::Result;

/// Entry in the blocklist with expiration
#[derive(Clone, Debug)]
struct BlockEntry {
    reason: String,
    expires: Instant,
    #[allow(dead_code)] // Stored for potential future use in severity-based decisions
    severity: u8,
    action_id: String,
}

/// State for rate limiting a source
#[derive(Clone, Debug)]
struct RateLimitState {
    count: u32,
    window_start: Instant,
    limit: u32,
    window: Duration,
    action_id: String,
}

/// State for verification requirements
#[derive(Clone, Debug)]
struct VerificationRequirement {
    challenge_type: ChallengeType,
    expires: Instant,
    action_id: String,
}

/// Enforcement status for a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementStatus {
    /// Whether the source is currently blocked
    pub blocked: bool,
    /// Whether the source is rate limited (hit the limit)
    pub rate_limited: bool,
    /// Whether verification is required
    pub verification_required: bool,
    /// Remaining requests in rate limit window (None if no limit)
    pub rate_limit_remaining: Option<u32>,
    /// Block reason if blocked
    pub block_reason: Option<String>,
    /// Time until block expires (in seconds)
    pub block_expires_in_secs: Option<u64>,
}

/// Mitigation engine with real enforcement capabilities
///
/// This engine maintains thread-safe state for:
/// - Blocklist with automatic expiration
/// - Rate limiting with sliding windows
/// - Verification requirements
#[derive(Clone)]
pub struct MitigationEngine {
    blocklist: Arc<DashMap<String, BlockEntry>>,
    rate_limits: Arc<DashMap<String, RateLimitState>>,
    verification_requirements: Arc<DashMap<String, VerificationRequirement>>,
    /// Default rate limit (requests per window)
    #[allow(dead_code)] // Available for future configuration API
    default_rate_limit: u32,
    /// Default rate limit window duration
    #[allow(dead_code)] // Available for future configuration API
    default_rate_window: Duration,
}

impl Default for MitigationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MitigationEngine {
    /// Create a new mitigation engine with default settings
    pub fn new() -> Self {
        Self {
            blocklist: Arc::new(DashMap::new()),
            rate_limits: Arc::new(DashMap::new()),
            verification_requirements: Arc::new(DashMap::new()),
            default_rate_limit: 10,
            default_rate_window: Duration::from_secs(60),
        }
    }

    /// Create a mitigation engine with custom rate limit defaults
    pub fn with_rate_limits(limit: u32, window: Duration) -> Self {
        Self {
            blocklist: Arc::new(DashMap::new()),
            rate_limits: Arc::new(DashMap::new()),
            verification_requirements: Arc::new(DashMap::new()),
            default_rate_limit: limit,
            default_rate_window: window,
        }
    }

    /// Check if a source is currently blocked
    ///
    /// Returns true if the source is in the blocklist and the block hasn't expired.
    /// Automatically removes expired entries.
    pub fn is_blocked(&self, source_id: &str) -> bool {
        if let Some(entry) = self.blocklist.get(source_id) {
            if entry.expires > Instant::now() {
                return true;
            }
            // Expired - remove it
            drop(entry);
            self.blocklist.remove(source_id);
        }
        false
    }

    /// Check rate limit for a source and increment counter
    ///
    /// Returns true if the request is allowed, false if rate limited.
    /// Automatically resets the window when it expires.
    pub fn check_rate_limit(&self, source_id: &str) -> bool {
        if let Some(mut state) = self.rate_limits.get_mut(source_id) {
            let now = Instant::now();

            // Reset if window expired
            if now.duration_since(state.window_start) > state.window {
                state.count = 1;
                state.window_start = now;
                return true;
            }

            // Check if over limit
            if state.count >= state.limit {
                return false;
            }

            state.count += 1;
            true
        } else {
            // No rate limit configured for this source
            true
        }
    }

    /// Check rate limit without incrementing counter (peek)
    pub fn peek_rate_limit(&self, source_id: &str) -> Option<(u32, u32)> {
        if let Some(state) = self.rate_limits.get(source_id) {
            let now = Instant::now();
            if now.duration_since(state.window_start) > state.window {
                // Window expired, would reset
                Some((0, state.limit))
            } else {
                Some((state.count, state.limit))
            }
        } else {
            None
        }
    }

    /// Check if verification is required for a source
    pub fn requires_verification(&self, source_id: &str) -> Option<ChallengeType> {
        if let Some(req) = self.verification_requirements.get(source_id) {
            if req.expires > Instant::now() {
                return Some(req.challenge_type.clone());
            }
            // Expired - remove it
            drop(req);
            self.verification_requirements.remove(source_id);
        }
        None
    }

    /// Get comprehensive enforcement status for a source
    pub fn get_enforcement_status(&self, source_id: &str) -> EnforcementStatus {
        let blocked = self.is_blocked(source_id);
        let block_info = if blocked {
            self.blocklist.get(source_id).map(|e| {
                let expires_in = e.expires.saturating_duration_since(Instant::now());
                (e.reason.clone(), expires_in.as_secs())
            })
        } else {
            None
        };

        let rate_info = self.peek_rate_limit(source_id);
        let rate_limited = rate_info
            .map(|(count, limit)| count >= limit)
            .unwrap_or(false);
        let rate_limit_remaining = rate_info.map(|(count, limit)| limit.saturating_sub(count));

        let verification_required = self.requires_verification(source_id).is_some();

        EnforcementStatus {
            blocked,
            rate_limited,
            verification_required,
            rate_limit_remaining,
            block_reason: block_info.as_ref().map(|(r, _)| r.clone()),
            block_expires_in_secs: block_info.map(|(_, s)| s),
        }
    }

    /// Execute a mitigation action with real enforcement
    pub async fn execute_action(
        &self,
        action: &MitigationAction,
        context: &ThreatContext,
    ) -> Result<String> {
        match action {
            MitigationAction::BlockRequest { reason } => {
                self.execute_block(context, reason).await
            }
            MitigationAction::RateLimitUser { duration } => {
                self.execute_rate_limit(context, *duration).await
            }
            MitigationAction::RequireVerification { challenge_type } => {
                self.execute_verification(context, challenge_type).await
            }
            MitigationAction::AlertHuman { priority } => {
                self.execute_alert(context, priority).await
            }
            MitigationAction::UpdateRules { new_patterns } => {
                self.execute_rule_update(context, new_patterns).await
            }
        }
    }

    /// Execute block action - actually adds source to blocklist
    async fn execute_block(&self, context: &ThreatContext, reason: &str) -> Result<String> {
        let action_id = uuid::Uuid::new_v4().to_string();

        // Calculate block duration based on severity
        let duration = match context.severity {
            0..=30 => Duration::from_secs(300),     // 5 minutes for low severity
            31..=70 => Duration::from_secs(3600),   // 1 hour for medium severity
            _ => Duration::from_secs(86400),        // 24 hours for high severity
        };

        // Actually add to blocklist
        self.blocklist.insert(
            context.source_id.clone(),
            BlockEntry {
                reason: reason.to_string(),
                expires: Instant::now() + duration,
                severity: context.severity,
                action_id: action_id.clone(),
            },
        );

        tracing::warn!(
            source_id = %context.source_id,
            reason = %reason,
            duration_secs = duration.as_secs(),
            severity = context.severity,
            action_id = %action_id,
            "Source BLOCKED - enforcement active"
        );

        metrics::counter!("mitigation.blocks.enforced").increment(1);
        metrics::gauge!("mitigation.blocklist.size").set(self.blocklist.len() as f64);

        Ok(action_id)
    }

    /// Execute rate limit action - actually applies rate limiting
    async fn execute_rate_limit(
        &self,
        context: &ThreatContext,
        duration: Duration,
    ) -> Result<String> {
        let action_id = uuid::Uuid::new_v4().to_string();

        // Calculate limit based on severity (lower for higher severity)
        let limit = match context.severity {
            0..=30 => 20,   // Lenient for low severity
            31..=70 => 10,  // Moderate for medium severity
            _ => 5,         // Strict for high severity
        };

        // Actually apply rate limit
        self.rate_limits.insert(
            context.source_id.clone(),
            RateLimitState {
                count: 0,
                window_start: Instant::now(),
                limit,
                window: duration,
                action_id: action_id.clone(),
            },
        );

        tracing::warn!(
            source_id = %context.source_id,
            duration_secs = duration.as_secs(),
            limit = limit,
            action_id = %action_id,
            "Rate limit ENFORCED"
        );

        metrics::counter!("mitigation.rate_limits.enforced").increment(1);
        metrics::gauge!("mitigation.rate_limits.active").set(self.rate_limits.len() as f64);

        Ok(action_id)
    }

    /// Execute verification requirement
    async fn execute_verification(
        &self,
        context: &ThreatContext,
        challenge: &ChallengeType,
    ) -> Result<String> {
        let action_id = uuid::Uuid::new_v4().to_string();

        // Verification requirement expires after 1 hour
        let duration = Duration::from_secs(3600);

        self.verification_requirements.insert(
            context.source_id.clone(),
            VerificationRequirement {
                challenge_type: challenge.clone(),
                expires: Instant::now() + duration,
                action_id: action_id.clone(),
            },
        );

        tracing::warn!(
            source_id = %context.source_id,
            challenge_type = ?challenge,
            action_id = %action_id,
            "Verification requirement ENFORCED"
        );

        metrics::counter!("mitigation.verifications.enforced").increment(1);

        Ok(action_id)
    }

    /// Execute human alert (this one remains log-based as alerts are external)
    async fn execute_alert(&self, context: &ThreatContext, priority: &AlertPriority) -> Result<String> {
        let action_id = uuid::Uuid::new_v4().to_string();

        tracing::warn!(
            source_id = %context.source_id,
            threat_id = %context.threat_id,
            priority = ?priority,
            action_id = %action_id,
            "ALERT: Security team notification required"
        );

        // In production, this would integrate with PagerDuty, Slack, etc.
        metrics::counter!("mitigation.alerts.triggered").increment(1);

        Ok(action_id)
    }

    /// Execute rule update
    async fn execute_rule_update(
        &self,
        _context: &ThreatContext,
        patterns: &[Pattern],
    ) -> Result<String> {
        let action_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            pattern_count = patterns.len(),
            action_id = %action_id,
            "Rule update applied"
        );

        // In production, this would update the detection engine rules
        metrics::counter!("mitigation.rule_updates.applied").increment(1);

        Ok(action_id)
    }

    /// Rollback a specific action
    pub fn rollback_action(&self, action_id: &str) -> Result<bool> {
        // Try to find and remove from blocklist
        let mut found = false;

        self.blocklist.retain(|_, entry| {
            if entry.action_id == action_id {
                found = true;
                tracing::info!(action_id = %action_id, "Block action rolled back");
                false // Remove this entry
            } else {
                true // Keep this entry
            }
        });

        if found {
            metrics::counter!("mitigation.rollbacks.blocks").increment(1);
            return Ok(true);
        }

        // Try rate limits
        self.rate_limits.retain(|_, state| {
            if state.action_id == action_id {
                found = true;
                tracing::info!(action_id = %action_id, "Rate limit rolled back");
                false
            } else {
                true
            }
        });

        if found {
            metrics::counter!("mitigation.rollbacks.rate_limits").increment(1);
            return Ok(true);
        }

        // Try verification requirements
        self.verification_requirements.retain(|_, req| {
            if req.action_id == action_id {
                found = true;
                tracing::info!(action_id = %action_id, "Verification requirement rolled back");
                false
            } else {
                true
            }
        });

        if found {
            metrics::counter!("mitigation.rollbacks.verifications").increment(1);
        }

        Ok(found)
    }

    /// Clear all mitigations for a source (unblock, remove rate limits, etc.)
    pub fn clear_source(&self, source_id: &str) {
        self.blocklist.remove(source_id);
        self.rate_limits.remove(source_id);
        self.verification_requirements.remove(source_id);

        tracing::info!(source_id = %source_id, "All mitigations cleared for source");
        metrics::counter!("mitigation.sources_cleared").increment(1);
    }

    /// Clean up expired entries (can be called periodically)
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut removed = 0;

        self.blocklist.retain(|_, entry| {
            if entry.expires <= now {
                removed += 1;
                false
            } else {
                true
            }
        });

        self.verification_requirements.retain(|_, req| {
            if req.expires <= now {
                removed += 1;
                false
            } else {
                true
            }
        });

        if removed > 0 {
            tracing::debug!(removed = removed, "Cleaned up expired mitigation entries");
            metrics::counter!("mitigation.expired_cleanup").increment(removed);
        }
    }

    /// Get statistics about current enforcement state
    pub fn stats(&self) -> EnforcementStats {
        EnforcementStats {
            blocked_sources: self.blocklist.len(),
            rate_limited_sources: self.rate_limits.len(),
            verification_required_sources: self.verification_requirements.len(),
        }
    }
}

/// Statistics about current enforcement state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementStats {
    pub blocked_sources: usize,
    pub rate_limited_sources: usize,
    pub verification_required_sources: usize,
}

/// Mitigation actions that can be taken against threats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MitigationAction {
    /// Block the threatening request
    BlockRequest { reason: String },

    /// Apply rate limiting to user/source
    RateLimitUser { duration: Duration },

    /// Require additional verification
    RequireVerification { challenge_type: ChallengeType },

    /// Alert human operator
    AlertHuman { priority: AlertPriority },

    /// Update detection rules
    UpdateRules { new_patterns: Vec<Pattern> },
}

impl MitigationAction {
    /// Execute mitigation action (legacy method - prefer using MitigationEngine)
    ///
    /// Note: This method only logs actions. For real enforcement,
    /// use `MitigationEngine::execute_action` instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use MitigationEngine::execute_action for real enforcement"
    )]
    pub async fn execute(&self, context: &ThreatContext) -> Result<String> {
        match self {
            MitigationAction::BlockRequest { reason } => {
                self.execute_block_legacy(context, reason).await
            }
            MitigationAction::RateLimitUser { duration } => {
                self.execute_rate_limit_legacy(context, *duration).await
            }
            MitigationAction::RequireVerification { challenge_type } => {
                self.execute_verification_legacy(context, challenge_type).await
            }
            MitigationAction::AlertHuman { priority } => {
                self.execute_alert_legacy(context, priority).await
            }
            MitigationAction::UpdateRules { new_patterns } => {
                self.execute_rule_update_legacy(context, new_patterns).await
            }
        }
    }

    /// Rollback mitigation action (legacy)
    #[deprecated(
        since = "0.2.0",
        note = "Use MitigationEngine::rollback_action for real rollback"
    )]
    pub fn rollback(&self, action_id: &str) -> Result<()> {
        tracing::info!("Rolling back action: {}", action_id);
        Ok(())
    }

    // Legacy implementations (log-only, kept for backward compatibility)
    async fn execute_block_legacy(&self, context: &ThreatContext, reason: &str) -> Result<String> {
        tracing::info!(
            "Blocking request from {} - Reason: {}",
            context.source_id,
            reason
        );
        let action_id = uuid::Uuid::new_v4().to_string();
        metrics::counter!("mitigation.blocks").increment(1);
        Ok(action_id)
    }

    async fn execute_rate_limit_legacy(
        &self,
        context: &ThreatContext,
        duration: Duration,
    ) -> Result<String> {
        tracing::info!("Rate limiting {} for {:?}", context.source_id, duration);
        let action_id = uuid::Uuid::new_v4().to_string();
        metrics::counter!("mitigation.rate_limits").increment(1);
        Ok(action_id)
    }

    async fn execute_verification_legacy(
        &self,
        context: &ThreatContext,
        challenge: &ChallengeType,
    ) -> Result<String> {
        tracing::info!(
            "Requiring {:?} verification for {}",
            challenge,
            context.source_id
        );
        let action_id = uuid::Uuid::new_v4().to_string();
        metrics::counter!("mitigation.verifications").increment(1);
        Ok(action_id)
    }

    async fn execute_alert_legacy(
        &self,
        context: &ThreatContext,
        priority: &AlertPriority,
    ) -> Result<String> {
        tracing::warn!(
            "Alerting security team - Priority: {:?} - Threat: {}",
            priority,
            context.threat_id
        );
        let action_id = uuid::Uuid::new_v4().to_string();
        metrics::counter!("mitigation.alerts").increment(1);
        Ok(action_id)
    }

    async fn execute_rule_update_legacy(
        &self,
        _context: &ThreatContext,
        patterns: &[Pattern],
    ) -> Result<String> {
        tracing::info!("Updating rules with {} new patterns", patterns.len());
        let action_id = uuid::Uuid::new_v4().to_string();
        metrics::counter!("mitigation.rule_updates").increment(1);
        Ok(action_id)
    }
}

/// Trait for mitigation implementations
#[async_trait::async_trait]
pub trait Mitigation: Send + Sync {
    /// Execute the mitigation
    async fn execute(&self, context: &ThreatContext) -> Result<MitigationOutcome>;

    /// Rollback the mitigation
    fn rollback(&self) -> Result<()>;
}

/// Context for mitigation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatContext {
    pub threat_id: String,
    pub source_id: String,
    pub threat_type: String,
    pub severity: u8,
    pub confidence: f64,
    pub metadata: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ThreatContext {
    fn default() -> Self {
        Self {
            threat_id: String::new(),
            source_id: String::new(),
            threat_type: String::new(),
            severity: 0,
            confidence: 0.0,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl ThreatContext {
    /// Create context from threat incident
    pub fn from_incident(incident: &ThreatIncident) -> Self {
        Self {
            threat_id: incident.id.clone(),
            source_id: format!("source_{}", incident.id),
            threat_type: format!("{:?}", incident.threat_type),
            severity: incident.severity,
            confidence: incident.confidence,
            metadata: HashMap::new(),
            timestamp: incident.timestamp,
        }
    }

    /// Add metadata to context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Outcome of mitigation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationOutcome {
    pub strategy_id: String,
    pub threat_type: String,
    pub features: HashMap<String, f64>,
    pub success: bool,
    pub actions_applied: Vec<String>,
    pub duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MitigationOutcome {
    /// Calculate effectiveness score
    pub fn effectiveness_score(&self) -> f64 {
        if self.success {
            // Higher score for faster mitigations
            let time_factor = 1.0 - (self.duration.as_millis() as f64 / 1000.0).min(1.0);
            0.7 + 0.3 * time_factor
        } else {
            0.0
        }
    }

    /// Check if outcome requires rollback
    pub fn requires_rollback(&self) -> bool {
        !self.success && !self.actions_applied.is_empty()
    }
}

/// Pattern for rule updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub features: HashMap<String, f64>,
}

/// Pattern type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    Signature,
    Anomaly,
    Behavioral,
    Statistical,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context(source_id: &str, severity: u8) -> ThreatContext {
        ThreatContext {
            threat_id: format!("threat-{}", uuid::Uuid::new_v4()),
            source_id: source_id.to_string(),
            threat_type: "test".to_string(),
            severity,
            confidence: 0.9,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_block_actually_blocks() {
        let engine = MitigationEngine::new();
        let context = create_test_context("test-source-1", 80);

        // Should not be blocked initially
        assert!(!engine.is_blocked(&context.source_id));

        // Execute block
        let action_id = engine
            .execute_block(&context, "test reason")
            .await
            .unwrap();
        assert!(!action_id.is_empty());

        // Should now be blocked
        assert!(engine.is_blocked(&context.source_id));

        // Verify stats
        let stats = engine.stats();
        assert_eq!(stats.blocked_sources, 1);
    }

    #[tokio::test]
    async fn test_rate_limit_enforced() {
        let engine = MitigationEngine::new();
        let context = create_test_context("test-source-2", 50); // Medium severity = 10 req limit

        // Apply rate limit
        engine
            .execute_rate_limit(&context, Duration::from_secs(60))
            .await
            .unwrap();

        // First 10 requests should pass
        for i in 0..10 {
            assert!(
                engine.check_rate_limit(&context.source_id),
                "Request {} should be allowed",
                i + 1
            );
        }

        // 11th request should fail (rate limited)
        assert!(
            !engine.check_rate_limit(&context.source_id),
            "Request 11 should be rate limited"
        );
    }

    #[tokio::test]
    async fn test_rate_limit_window_reset() {
        let engine = MitigationEngine::with_rate_limits(5, Duration::from_millis(50));
        let context = create_test_context("test-source-3", 80); // High severity = 5 req limit

        // Apply rate limit with very short window
        engine
            .execute_rate_limit(&context, Duration::from_millis(50))
            .await
            .unwrap();

        // Use up the rate limit
        for _ in 0..5 {
            engine.check_rate_limit(&context.source_id);
        }

        // Should be rate limited
        assert!(!engine.check_rate_limit(&context.source_id));

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be allowed again (window reset)
        assert!(engine.check_rate_limit(&context.source_id));
    }

    #[tokio::test]
    async fn test_verification_required() {
        let engine = MitigationEngine::new();
        let context = create_test_context("test-source-4", 60);

        // Should not require verification initially
        assert!(engine.requires_verification(&context.source_id).is_none());

        // Execute verification requirement
        engine
            .execute_verification(&context, &ChallengeType::Captcha)
            .await
            .unwrap();

        // Should now require verification
        let challenge = engine.requires_verification(&context.source_id);
        assert!(challenge.is_some());
        assert!(matches!(challenge.unwrap(), ChallengeType::Captcha));
    }

    #[tokio::test]
    async fn test_enforcement_status() {
        let engine = MitigationEngine::new();
        let context = create_test_context("test-source-5", 70);

        // Initial status should be clean
        let status = engine.get_enforcement_status(&context.source_id);
        assert!(!status.blocked);
        assert!(!status.rate_limited);
        assert!(!status.verification_required);

        // Block the source
        engine
            .execute_block(&context, "suspicious activity")
            .await
            .unwrap();

        // Status should reflect block
        let status = engine.get_enforcement_status(&context.source_id);
        assert!(status.blocked);
        assert!(status.block_reason.is_some());
        assert!(status.block_expires_in_secs.is_some());
    }

    #[tokio::test]
    async fn test_rollback_action() {
        let engine = MitigationEngine::new();
        let context = create_test_context("test-source-6", 50);

        // Block the source
        let action_id = engine
            .execute_block(&context, "test block")
            .await
            .unwrap();

        // Verify blocked
        assert!(engine.is_blocked(&context.source_id));

        // Rollback the action
        let rolled_back = engine.rollback_action(&action_id).unwrap();
        assert!(rolled_back);

        // Should no longer be blocked
        assert!(!engine.is_blocked(&context.source_id));
    }

    #[tokio::test]
    async fn test_clear_source() {
        let engine = MitigationEngine::new();
        let context = create_test_context("test-source-7", 60);

        // Apply multiple mitigations
        engine
            .execute_block(&context, "test")
            .await
            .unwrap();
        engine
            .execute_rate_limit(&context, Duration::from_secs(60))
            .await
            .unwrap();
        engine
            .execute_verification(&context, &ChallengeType::Captcha)
            .await
            .unwrap();

        // Verify all applied
        assert!(engine.is_blocked(&context.source_id));
        let stats = engine.stats();
        assert!(stats.blocked_sources > 0);
        assert!(stats.rate_limited_sources > 0);
        assert!(stats.verification_required_sources > 0);

        // Clear the source
        engine.clear_source(&context.source_id);

        // All mitigations should be cleared
        assert!(!engine.is_blocked(&context.source_id));
        assert!(engine.requires_verification(&context.source_id).is_none());
    }

    #[tokio::test]
    async fn test_block_duration_by_severity() {
        let engine = MitigationEngine::new();

        // Low severity (0-30) should get 5 min block
        let low_context = create_test_context("low-sev", 20);
        engine
            .execute_block(&low_context, "low severity")
            .await
            .unwrap();

        // Medium severity (31-70) should get 1 hour block
        let med_context = create_test_context("med-sev", 50);
        engine
            .execute_block(&med_context, "medium severity")
            .await
            .unwrap();

        // High severity (71+) should get 24 hour block
        let high_context = create_test_context("high-sev", 90);
        engine
            .execute_block(&high_context, "high severity")
            .await
            .unwrap();

        // All should be blocked
        assert!(engine.is_blocked(&low_context.source_id));
        assert!(engine.is_blocked(&med_context.source_id));
        assert!(engine.is_blocked(&high_context.source_id));

        // Check the expiration times are different
        let low_status = engine.get_enforcement_status(&low_context.source_id);
        let med_status = engine.get_enforcement_status(&med_context.source_id);
        let high_status = engine.get_enforcement_status(&high_context.source_id);

        // Low < Med < High expiration
        assert!(
            low_status.block_expires_in_secs.unwrap()
                < med_status.block_expires_in_secs.unwrap()
        );
        assert!(
            med_status.block_expires_in_secs.unwrap()
                < high_status.block_expires_in_secs.unwrap()
        );
    }

    #[tokio::test]
    async fn test_execute_action_through_engine() {
        let engine = MitigationEngine::new();
        let context = create_test_context("test-source-8", 70);

        // Execute block action through engine
        let action = MitigationAction::BlockRequest {
            reason: "test block".to_string(),
        };
        let action_id = engine.execute_action(&action, &context).await.unwrap();

        // Should be blocked
        assert!(engine.is_blocked(&context.source_id));
        assert!(!action_id.is_empty());
    }

    #[test]
    fn test_effectiveness_score() {
        let outcome = MitigationOutcome {
            strategy_id: "test".to_string(),
            threat_type: "anomaly".to_string(),
            features: HashMap::new(),
            success: true,
            actions_applied: vec!["action-1".to_string()],
            duration: Duration::from_millis(50),
            timestamp: chrono::Utc::now(),
        };

        let score = outcome.effectiveness_score();
        assert!(score > 0.7);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_context_creation() {
        let incident = crate::meta_learning::ThreatIncident {
            id: "test-3".to_string(),
            threat_type: crate::meta_learning::ThreatType::Anomaly(0.85),
            severity: 7,
            confidence: 0.9,
            timestamp: chrono::Utc::now(),
        };

        let context = ThreatContext::from_incident(&incident);
        assert_eq!(context.threat_id, "test-3");
        assert_eq!(context.severity, 7);
    }

    #[test]
    fn test_default_context() {
        let context = ThreatContext::default();
        assert!(context.threat_id.is_empty());
        assert!(context.source_id.is_empty());
        assert_eq!(context.severity, 0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use std::sync::Arc;
        use tokio::task::JoinSet;

        let engine = Arc::new(MitigationEngine::new());
        let mut tasks = JoinSet::new();

        // Spawn multiple tasks that access the engine concurrently
        for i in 0..10 {
            let engine = Arc::clone(&engine);
            tasks.spawn(async move {
                let context = create_test_context(&format!("concurrent-{}", i), 50);
                engine
                    .execute_block(&context, "concurrent test")
                    .await
                    .unwrap();
                engine.is_blocked(&context.source_id)
            });
        }

        // All should complete successfully and be blocked
        while let Some(result) = tasks.join_next().await {
            assert!(result.unwrap());
        }

        // Should have 10 blocked sources
        assert_eq!(engine.stats().blocked_sources, 10);
    }
}
