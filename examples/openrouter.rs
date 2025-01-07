use midstream::{Midstream, HyprSettings, HyprServiceImpl, StreamProcessor, LLMClient};
use futures::stream::{BoxStream, Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use std::pin::Pin;
use eventsource_stream::Eventsource;
use dotenv::dotenv;

struct OpenRouterClient {
    client: Client,
    api_key: String,
}

impl OpenRouterClient {
    fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    async fn stream_completion(&self, prompt: &str) -> BoxStream<'static, String> {
        let url = "https://openrouter.ai/api/v1/chat/completions";
        
        let payload = json!({
            "model": "anthropic/claude-2",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "stream": true
        });

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "http://localhost:3000")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                event.map_or_else(
                    |e| format!("Error: {}", e),
                    |event| {
                        if event.data == "[DONE]" {
                            String::new()
                        } else {
                            // Parse the SSE data as JSON
                            if let Ok(value) = serde_json::from_str::<Value>(&event.data) {
                                // Extract the content from the completion
                                value["choices"][0]["delta"]["content"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string()
                            } else {
                                String::new()
                            }
                        }
                    }
                )
            })
            .filter(|s| !s.is_empty());

        Box::pin(stream)
    }
}

impl LLMClient for OpenRouterClient {
    fn stream(&self) -> BoxStream<'static, String> {
        let prompt = "Tell me a short story about a robot learning to paint. Make it emotional and stream it word by word.";
        
        let future = self.stream_completion(prompt);
        Box::pin(async move {
            future.await
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    
    // Get API key from environment
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set in .env file");

    // Initialize settings
    let settings = HyprSettings::new()?;
    
    // Create hyprstream service
    let hypr_service = HyprServiceImpl::new(&settings).await?;
    
    // Create OpenRouter client
    let llm_client = OpenRouterClient::new(api_key);
    
    // Initialize Midstream
    let midstream = Midstream::new(
        Box::new(llm_client),
        Box::new(hypr_service),
    );
    
    println!("\nStreaming story from Claude-2...\n");

    // Process stream
    let messages = midstream.process_stream().await?;
    
    println!("\nFinal story:");
    for msg in &messages {
        print!("{}", msg.content);
    }
    println!("\n");
    
    // Get metrics
    let metrics = midstream.get_metrics().await;
    println!("\nMetrics collected:");
    for metric in &metrics {
        println!("- Token count: {}", metric.value);
        println!("  Labels: {:?}", metric.labels);
        println!();
    }
    
    // Get average sentiment for last 5 minutes
    let avg = midstream.get_average_sentiment(Duration::from_secs(300)).await?;
    println!("\nAverage tokens per message: {:.2}", avg);
    
    Ok(())
}