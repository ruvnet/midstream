//! MidStream: Real-Time Large Language Model Streaming Platform
//!
//! This library provides functionality for real-time LLM response streaming,
//! inflight data analysis, and integration with external tools.
//!
//! # Example
//!
//! ```rust,no_run
//! use midstream::{Midstream, HyprSettings, HyprServiceImpl, StreamProcessor, LLMClient};
//! use bytes::Bytes;
//! use futures::stream::BoxStream;
//! use futures::stream::iter;
//! use std::time::Duration;
//!
//! // Example LLM client implementation
//! struct ExampleLLMClient;
//!
//! impl LLMClient for ExampleLLMClient {
//!     fn stream(&self) -> BoxStream<'static, Bytes> {
//!         Box::pin(iter(vec![
//!             Bytes::from_static(b"Processing"),
//!             Bytes::from_static(b"the"),
//!             Bytes::from_static(b"stream"),
//!         ]))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize settings
//!     let settings = HyprSettings::new()?;
//!     
//!     // Create hyprstream service
//!     let hypr_service = HyprServiceImpl::new(&settings).await?;
//!     
//!     // Create LLM client
//!     let llm_client = ExampleLLMClient;
//!     
//!     // Initialize Midstream
//!     let midstream = Midstream::new(
//!         Box::new(llm_client),
//!         Box::new(hypr_service),
//!     );
//!     
//!     // Process stream
//!     let messages = midstream.process_stream().await?;
//!     println!("Processed messages: {:?}", messages);
//!     
//!     // Get metrics
//!     let metrics = midstream.get_metrics().await;
//!     println!("Collected metrics: {:?}", metrics);
//!     
//!     // Get average sentiment for last 5 minutes
//!     let avg = midstream.get_average_sentiment(Duration::from_secs(300)).await?;
//!     println!("Average sentiment: {}", avg);
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod hypr_service;
pub mod midstream;
pub mod tests;

// `lean_agentic` is the legacy in-tree subsystem that ADR-0005 retires.
// It currently fails to compile and duplicates functionality in the
// `midstreamer-*` workspace crates. Gated off-by-default until the
// dedup refactor lands; consumers wanting the old API still build with
// `--features lean-agentic` and accept the broken-build risk.
#[cfg(feature = "lean-agentic")]
pub mod lean_agentic;

pub use config::HyprSettings;
pub use hypr_service::HyprServiceImpl;
pub use midstream::{
    AggregateFunction, HyprService, Intent, LLMClient, LLMMessage, MetricRecord, Midstream,
    StreamProcessor, TimeWindow, ToolIntegration,
};

// Lean Agentic Learning System exports — gated behind the same feature.
// Once ADR-0005's dedup ships, the canonical home for these types is
// the published `midstreamer-*` crates; this re-export block goes away.
#[cfg(feature = "lean-agentic")]
pub use lean_agentic::{
    Action, AdaptationStrategy, AgentState, AgenticLoop, Context as AgentContext, Entity,
    FormalReasoner, KnowledgeGraph, LeanAgenticConfig, LeanAgenticSystem, LearningSignal,
    Observation, OnlineModel, Plan, Proof, ProofStep, Relation, Reward, StreamLearner, Theorem,
};
