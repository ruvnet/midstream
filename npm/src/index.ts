/**
 * MidStream - Real-time LLM Streaming with Lean Agentic Learning
 *
 * Main exports for npm package
 */

export { MidStreamAgent } from './agent.js';
export { WebSocketStreamServer, SSEStreamServer, HTTPStreamingClient } from './streaming.js';
export { MidStreamMCPServer } from './mcp-server.js';
export {
  OpenAIRealtimeClient,
  AgenticFlowProxyClient,
  createDefaultSessionConfig,
  audioToBase64,
  base64ToAudio
} from './openai-realtime.js';

// Re-export types
export type {
  AgentConfig,
  AnalysisResult,
  BehaviorAnalysis,
} from './agent.js';

export type {
  RealtimeConfig,
  SessionConfig,
  ConversationItem,
  RealtimeMessage,
} from './openai-realtime.js';
