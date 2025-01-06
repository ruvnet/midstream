# MidStream

**Midstream** is a cutting-edge platform designed for real-time Large Language Model (LLM) streaming, enabling **inflight analysis** of data as it streams from the LLM. This capability allows for **instant decision-making** and **dynamic interactions** before the entire stream is complete. By integrating intelligent agents equipped with specialized tools and inflight logic, Midstream empowers applications to respond proactively, enhancing efficiency and user experience across various domains.

## Key Features

### 🔄 Real-Time LLM Streaming
- **Seamless Integration**: Connect effortlessly with popular LLMs to stream generated content in real time.
- **Low Latency**: Deliver instantaneous responses by processing data on-the-fly, minimizing wait times.

### 🧠 Inflight Data Analysis
- **Dynamic Processing**: Analyze streaming data as it arrives, enabling immediate insights and actions.
- **Contextual Understanding**: Utilize advanced algorithms to comprehend and interpret data contextually during the streaming process.

### 🤖 Intelligent Agents with Tool Integration
- **Adaptive Agents**: Deploy agents that can interact with external tools and APIs based on real-time analysis.
- **Toolkits Integration**: Integrate with calendars, weather services, databases, and more to enrich interactions and automate tasks.

### ⚡ Decision-Making Before Stream Completion
- **Proactive Responses**: Make informed decisions and take actions before the full stream is received, enhancing responsiveness.
- **Conditional Logic**: Implement complex logic paths that trigger based on specific data patterns or keywords detected during streaming.

### 🌐 Scalable Architecture
- **Distributed Processing**: Handle multiple concurrent streams efficiently, ensuring scalability as demand grows.
- **Fault Tolerance**: Maintain high availability and reliability through robust error handling and recovery mechanisms.

### 🔒 Secure Operations
- **Data Privacy**: Ensure all streamed and processed data is handled with the highest security standards.
- **Authentication & Authorization**: Implement strict access controls to protect sensitive information and system integrity.

## Practical Applications

### 1. **Real-Time Customer Support Chatbots**
Enhance customer interactions by providing instant, context-aware responses. Agents can schedule appointments, fetch user data, and integrate with CRM systems—all while engaging in a natural conversation.

### 2. **Financial Trading Assistants**
Analyze market data streams in real time to identify trading opportunities. Agents can execute trades, adjust portfolios, and provide insights based on live data without waiting for the entire data stream to process.

### 3. **Healthcare Monitoring Systems**
Stream patient data continuously and analyze it inflight to detect anomalies or critical conditions. Agents can alert medical staff, adjust monitoring parameters, and integrate with electronic health records instantly.

### 4. **Smart Home Automation**
Respond to user commands and environmental data in real time. Agents can control lighting, climate, security systems, and other smart devices based on ongoing interactions and sensor data streams.

### 5. **Content Moderation for Live Streaming Platforms**
Monitor live chat and content streams for inappropriate behavior or violations. Agents can remove offensive content, issue warnings, and take preventive actions without interrupting the live experience.

---

**Midstream** revolutionizes the way applications interact with LLMs by providing the tools necessary for real-time processing and intelligent decision-making. Whether enhancing user interactions, automating complex tasks, or ensuring timely responses, Midstream offers a robust foundation for building responsive and intelligent systems.
## Project Structure

```
real-time-agent/
├─ .env
├─ Dockerfile
├─ install.sh
├─ Cargo.toml
├─ src/
│  ├─ main.rs
│  ├─ router.rs
│  ├─ handlers.rs
│  └─ midstream.rs
└─ README.md
```

---

## 1. `.env`

This file holds environment variables for your application, such as database credentials and API keys.

```bash
# .env

# Midstream Configuration
MIDSTREAM_ENGINE_USERNAME=postgres
MIDSTREAM_ENGINE_PASSWORD=secret
MIDSTREAM_BACKEND_URL=postgresql://localhost:5432/metrics
MIDSTREAM_DRIVER_PATH=/usr/local/lib/libadbc_driver_postgresql.so

# External Services
CALENDAR_API_URL=https://api.calendar-service.com
CALENDAR_API_KEY=your_calendar_api_key
WEATHER_API_URL=https://api.weather-service.com
WEATHER_API_KEY=your_weather_api_key

# Server Configuration
SERVER_PORT=8080
```

> **Security Note:** Ensure that `.env` is added to your `.gitignore` to prevent sensitive information from being committed to version control.

---

## 2. `install.sh`

A script to install Rust (if not already installed) and build the project.

```bash
#!/usr/bin/env bash
set -e

# 1. Install Rust if not present (optional)
if ! command -v cargo &> /dev/null
then
    echo "Rust (cargo) not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# 2. Install dependencies
echo "Installing dependencies..."
cargo install --path .

echo "Build complete!"
```

> **Usage:** Make the script executable with `chmod +x install.sh` and run it using `./install.sh`.

---

## 3. `Dockerfile`

A Docker setup for building and running the application.

```Dockerfile
# Dockerfile

# Stage 1: Build
FROM rust:1.71 as builder

WORKDIR /app

# Install system dependencies for Midstream
RUN apt-get update && apt-get install -y libssl-dev pkg-config

# Copy source code
COPY . .

# Build the project
RUN ["./install.sh"]

# Stage 2: Run
FROM debian:bullseye-slim

WORKDIR /app

# Install necessary libraries for runtime
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*

# Copy build artifact from builder
COPY --from=builder /app/target/release/real-time-agent /usr/local/bin/real-time-agent

# Copy environment variables
COPY .env .env

# Expose the service port
EXPOSE 8080

# Run the application
CMD ["real-time-agent"]
```

> **Note:** Adjust `libssl1.1` based on your application's runtime requirements.

---

## 4. `Cargo.toml`

Defines the project's dependencies and metadata.

```toml
[package]
name = "real-time-agent"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
hyper = { version = "0.14", features = ["full"] }
routerify = "2.0"
dotenv = "0.15"
midstream = { git = "https://github.com/ruvnet/midstream.git", branch = "main" } # Assuming Midstream is available via Git
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
```

> **Dependencies Explained:**
> - **tokio:** Asynchronous runtime.
> - **hyper:** HTTP server.
> - **routerify:** Routing middleware.
> - **dotenv:** Load environment variables from `.env`.
> - **midstream:** Real-time data ingestion and processing.
> - **serde & serde_json:** Serialization/deserialization.
> - **reqwest:** HTTP client for external API calls.

---

## 5. `src/main.rs`

The main entry point of the application. It loads environment variables, sets up the router, initializes Midstream, and starts the server.

```rust
// src/main.rs

mod router;
mod handlers;
mod midstream;

use dotenv::dotenv;
use std::net::SocketAddr;
use hyper::Server;
use routerify::RouterService;
use real_time_agent::router::create_router;
use real_time_agent::midstream::Midstream;

#[tokio::main]
async fn main() {
    dotenv().ok(); // Load variables from .env

    // Initialize Midstream
    let midstream = Midstream::new().await.expect("Failed to initialize Midstream");

    // Create the main router with Midstream instance
    let router = create_router(midstream);

    // Wrap the router in a Hyper service
    let service = RouterService::new(router).expect("Failed to create RouterService");

    // Define socket address
    let port: u16 = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a number");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Start the HTTP server
    println!("Server listening on http://{}", addr);

    if let Err(err) = Server::bind(&addr).serve(service).await {
        eprintln!("Server error: {}", err);
    }
}
```

---

## 6. `src/router.rs`

Defines the application's routes and middleware using **routerify**. Integrates **Midstream** for real-time data ingestion and analysis.

```rust
// src/router.rs

use routerify::{Router, Middleware, Request, Response, Body};
use hyper::StatusCode;
use std::convert::Infallible;
use crate::handlers::{stream_llm_response, fallback_handler};
use crate::midstream::Midstream;

pub fn create_router(midstream: Midstream) -> Router<Body, Infallible> {
    Router::builder()
        // Pre-middleware: analyze inflight data using Midstream
        .middleware(Middleware::pre(move |req| {
            let midstream = midstream.clone();
            async move { 
                real_time_agent::midstream::analyze_and_integrate_middleware(req, midstream).await 
            }
        }))
        // Routes
        .get("/chat", stream_llm_response)
        // Fallback for not-found or other
        .any(fallback_handler)
        .build()
        .unwrap()
}
```

---

## 7. `src/handlers.rs`

Contains handler functions for individual routes, including the simulated LLM streaming response.

```rust
// src/handlers.rs

use hyper::{Body, Response, StatusCode};
use std::convert::Infallible;
use tokio::time::{sleep, Duration};

/// Simulate streaming LLM response
pub async fn stream_llm_response(_req: hyper::Request<Body>) 
    -> Result<Response<Body>, Infallible> 
{
    // Create a channel to stream data
    let (mut sender, body) = Body::channel();

    tokio::spawn(async move {
        let simulated_chunks = vec![
            "I've scheduled a meeting with John for next Wednesday at 10 AM. ",
            "The weather forecast for that day is sunny with a high of 75°F."
        ];

        for chunk in simulated_chunks {
            if sender.send_data(chunk.as_bytes().to_vec()).await.is_err() {
                break;
            }
            // Simulate delay between partial outputs
            sleep(Duration::from_millis(500)).await;
        }
    });

    Ok(Response::new(body))
}

/// Fallback handler for unmatched routes
pub async fn fallback_handler(_: hyper::Request<Body>) 
    -> Result<Response<Body>, Infallible> 
{
    let mut resp = Response::new(Body::from("Not Found"));
    *resp.status_mut() = StatusCode::NOT_FOUND;
    Ok(resp)
}
```

> **Note:** In a real-world scenario, you would replace the `simulated_chunks` with actual streaming responses from an LLM API.

---

## 8. `src/midstream.rs`

Implements **Midstream** integration for real-time data ingestion and inflight analysis. This module initializes Midstream and provides middleware for analyzing streaming data to detect intents and trigger tool integrations.

```rust
// src/midstream.rs

use midstream::MidstreamClient; // Hypothetical Midstream client
use hyper::{Body, Request};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::Deserialize;

#[derive(Clone)]
pub struct Midstream {
    client: Arc<Mutex<MidstreamClient>>,
}

impl Midstream {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize Midstream client with configurations from .env
        let backend_url = std::env::var("MIDSTREAM_BACKEND_URL")
            .expect("MIDSTREAM_BACKEND_URL must be set");
        let username = std::env::var("MIDSTREAM_ENGINE_USERNAME")
            .expect("MIDSTREAM_ENGINE_USERNAME must be set");
        let password = std::env::var("MIDSTREAM_ENGINE_PASSWORD")
            .expect("MIDSTREAM_ENGINE_PASSWORD must be set");
        let driver_path = std::env::var("MIDSTREAM_DRIVER_PATH")
            .expect("MIDSTREAM_DRIVER_PATH must be set");

        let client = MidstreamClient::new(&backend_url, &username, &password, &driver_path).await?;

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }
}

/// Middleware for analyzing and integrating inflight data
pub async fn analyze_and_integrate_middleware(
    req: Request<Body>,
    midstream: Midstream,
) -> Result<Request<Body>, std::convert::Infallible> {
    // For demonstration, assume that the request body contains streamed LLM responses
    // In a real implementation, integrate with Midstream's data processing pipeline

    // Clone necessary data
    let body = req.body().clone();
    
    // Process the body in a background task
    let midstream_clone = midstream.clone();
    tokio::spawn(async move {
        // Example: Extract text from the body and analyze
        if let Ok(full_body) = hyper::body::to_bytes(body).await {
            let text = String::from_utf8_lossy(&full_body);
            if text.contains("schedule a meeting") {
                // Trigger calendar API call
                println!("[MIDSTREAM] Detected intent: schedule a meeting");
                // Implement actual API call here
            }
            if text.contains("check weather") {
                // Trigger weather API call
                println!("[MIDSTREAM] Detected intent: check weather");
                // Implement actual API call here
            }

            // Optionally, store or process data with Midstream
            // Example: midstream_clone.client.lock().await.store_metric(...).await;
        }
    });

    // Pass the request through unchanged
    Ok(req)
}
```

> **Notes:**
> - The `MidstreamClient` and its methods are **hypothetical**. You should replace them with actual implementations based on Midstream's API.
> - This middleware asynchronously processes the request body to detect intents and trigger corresponding actions.
> - In a real-world scenario, you would handle streaming data more gracefully, possibly integrating with Midstream's real-time data processing features.

---

## 9. `README.md`

Provides instructions on how to set up, run, and extend the application, along with acknowledgement of the Hyprstream project.

```markdown
# Real-Time Conversational AI Assistant

This is a **minimal** Rust-based example demonstrating:
- Streaming LLM responses
- Inflight data analysis with **Midstream**
- Dynamic tool integrations (calendar, weather, database operations)
- Containerization with Docker

## Acknowledgement

This project was inspired by and acknowledges the [Hyprstream](https://github.com/yourusername/hyprstream) project, which provided foundational concepts for real-time data ingestion and processing. Special thanks to the Hyprstream contributors for their valuable work.

## Quick Start (Local)

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/real-time-agent.git
cd real-time-agent
```

### 2. Set Up Environment Variables

Create a `.env` file in the root directory based on the provided template.

```bash
cp .env.example .env
```

Populate `.env` with necessary credentials and API keys.

### 3. Install Dependencies and Build

Make the install script executable and run it.

```bash
chmod +x install.sh
./install.sh
```

### 4. Run the Application

```bash
cargo run --release
```

### 5. Test the Streaming Endpoint

In a new terminal, execute:

```bash
curl -N http://localhost:8080/chat
```

You should see partial outputs streaming with ~500ms delay.

## Docker

### 1. Build the Docker Image

```bash
docker build -t real-time-agent:latest .
```

### 2. Run the Docker Container

Ensure your `.env` file is properly configured.

```bash
docker run -it --rm \
  -p 8080:8080 \
  --env-file .env \
  real-time-agent:latest
```

### 3. Test the Streaming Endpoint

```bash
curl -N http://localhost:8080/chat
```

## Extending the Application

### 1. Midstream Integration

- **Initialize Midstream:** Modify `src/midstream.rs` to properly initialize and interact with Midstream's APIs.
- **Real-Time Metrics:** Define and compute metrics (e.g., running sums, counts, averages) using Midstream as data streams in.
- **Caching:** Utilize Midstream's caching mechanisms to store frequently accessed data for low-latency access.

### 2. External APIs

- **Calendar API:** Implement actual API calls to schedule meetings.
- **Weather API:** Integrate with a weather service to fetch real-time forecasts.
- **Example:**

  ```rust
  // Inside analyze_and_integrate_middleware
  if text.contains("schedule a meeting") {
      // Call Calendar API
      schedule_meeting().await;
  }
  if text.contains("check weather") {
      // Call Weather API
      check_weather().await;
  }
  ```

### 3. ADBC Integration

- **Database Operations:** Use Midstream's ADBC drivers to interact with databases like PostgreSQL, Redis, or Snowflake.
- **Example:**

  ```rust
  // Inside midstream.rs
  midstream_clone.client.lock().await.store_metric(metric).await;
  ```

### 4. LLM Integration

- **Replace Simulated Responses:** Integrate with an actual LLM API (e.g., OpenAI GPT-4) to stream real-time responses.
- **Handle Streaming Data:** Ensure that LLM responses are properly streamed to clients and processed by Midstream.

## Implementation Considerations

- **Streaming:** Ensure that LLM responses are streamed in real-time using asynchronous programming with `tokio`.
- **Real-Time Analysis:** Utilize Midstream's capabilities to perform inflight data analysis, enabling the detection of actionable intents during the streaming process.
- **Tool Integration:** Implement seamless integrations with external tools and APIs to enrich responses dynamically based on detected intents.
- **Intelligent Caching:** Leverage Midstream's caching mechanisms to store frequently accessed data, reducing latency for repeated queries.
- **ADBC Integration:** Use Midstream's ADBC compatibility to interact with various backend datastores, ensuring data consistency and flexibility.
- **Error Handling:** Implement robust error handling to manage failures in API calls, database interactions, or streaming processes.
- **Security:** Ensure secure communication with external APIs using HTTPS and proper authentication mechanisms. Protect sensitive data by securely managing environment variables and credentials.
- **Scalability:** Design the application to handle multiple concurrent conversations efficiently, leveraging Rust's concurrency features and Midstream's scalable architecture.

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
```

---

## Summary of How It Works

1. **Client Requests `/chat`:** A client sends a `GET` request to `/chat`.
2. **LLM Response Streaming:** The server streams a simulated LLM response in chunks with delays to mimic real-time generation.
3. **Midstream Middleware:** As each chunk is processed, the Midstream middleware analyzes the content for specific intents (e.g., "schedule a meeting", "check weather").
4. **Tool Integration:**
   - If an intent is detected, the middleware triggers corresponding external API calls (e.g., Calendar API, Weather API).
   - The fetched data can be integrated into the streaming response or used to perform backend operations.
5. **Streamed Response to Client:** The client receives the enriched response in real-time as the LLM streams the data.

---

## Final Notes

- **Midstream Integration:** This example assumes that Midstream provides a client library (`MidstreamClient`) for Rust. You should refer to Midstream's [documentation](https://github.com/ruvnet/midstream) for accurate integration details.
- **External Service Clients:** Implement robust clients for external APIs with proper error handling and authentication.
- **LLM Integration:** Replace the simulated LLM responses with actual streaming data from an LLM provider. Ensure that the streaming data is compatible with the middleware's processing logic.
- **Security Best Practices:** Always secure your application by managing secrets properly, validating and sanitizing inputs, and following best practices for authentication and authorization.

Feel free to expand upon this foundation to build a robust, scalable, and intelligent conversational AI assistant tailored to your specific needs. If you need further assistance with specific integrations, configurations, or enhancements, feel free to ask!
