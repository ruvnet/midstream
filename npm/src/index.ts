/**
 * MidStream - Real-time LLM Streaming with Lean Agentic Learning
 *
 * Main exports for npm package
 */

export { MidStreamAgent } from './agent.js';
export { WebSocketStreamServer, SSEStreamServer, HTTPStreamingClient } from './streaming.js';
export { MidStreamMCPServer } from './mcp-server.js';

// Re-export types
export type {
  AgentConfig,
  AnalysisResult,
  BehaviorAnalysis,
} from './agent.js';
