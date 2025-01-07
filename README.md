# MidStream: Real-Time Large Language Model Streaming Platform
This project demonstrates a **minimal** Rust-based implementation of a **Real-Time Conversational AI Assistant** with **Dynamic Tool Integration** using **MidStream**. It showcases how to stream LLM responses in real time, perform inflight data analysis, and integrate with external tools like calendars and weather services. The project leverages **Routerify**, an open-source routing library in Rust, to handle HTTP requests and manage streaming data efficiently.

## Acknowledgement

This project was inspired by and acknowledges the [HyprStream](https://github.com/yourusername/hyprstream) project, which provided foundational concepts for real-time data ingestion and processing. Special thanks to the HyprStream contributors for their valuable work.

## Key Features

- **Real-Time LLM Streaming**: Stream responses from Large Language Models with low latency.
- **Inflight Data Analysis**: Analyze streaming data on-the-fly to detect intents and make decisions before the stream completes.
- **Intelligent Agents**: Deploy agents that interact with external tools and APIs based on real-time analysis.
- **Routerify Integration**: Utilize Routerify for efficient routing and middleware support in handling streaming data.
- **Scalable and Secure**: Designed to handle multiple concurrent streams with robust security measures.

### Table of Contents
1. [Introduction](#introduction)
2. [Key Features](#key-features)
3. [Architecture Overview](#architecture-overview)
4. [Project Structure](#project-structure)
5. [Configuration Files](#configuration-files)
6. [Installation Script (`install.sh`)](#installation-script-installsh)
7. [Docker Configuration (`Dockerfile`)](#docker-configuration-dockerfile)
8. [Rust Project Configuration (`Cargo.toml`)](#rust-project-configuration-cargotoml)
9. [Source Code](#source-code)
    - [1. `main.rs`](#1-mainrs)
    - [2. `router.rs`](#2-routerrs)
    - [3. `handlers.rs`](#3-handlersrs)
    - [4. `midstream.rs`](#4-midstreamrs)
10. [README](#readme)
11. [Acknowledgement](#acknowledgement)
12. [Practical Applications](#practical-applications)
13. [Implementation Considerations](#implementation-considerations)
14. [License](#license)

---

## Introduction

**MidStream** is a cutting-edge platform designed for **real-time Large Language Model (LLM) streaming**, enabling **inflight analysis** of data as it streams from the LLM. This capability allows for **instant decision-making** and **dynamic interactions** before the entire stream is complete. By integrating intelligent agents equipped with specialized tools and inflight logic, MidStream empowers applications to respond proactively, enhancing efficiency and user experience across various domains.

Central to MidStream's architecture is the use of an **open-source router** implemented in Rust using **Routerify**. This robust routing system ensures high-performance handling of HTTP requests and seamless management of streaming data, making MidStream an ideal choice for building scalable and responsive conversational AI assistants.

---

## Key Features

### 🔄 Real-Time LLM Streaming
- **Seamless Integration**: Connect effortlessly with popular LLMs to stream generated content in real time.
- **Low Latency**: Deliver instantaneous responses by processing data on-the-fly, minimizing wait times.
- **Routerify-Powered Routing**: Utilize **Routerify**, a high-performance routing library in Rust, to manage and stream data efficiently.

### 🧠 Inflight Data Analysis
- **Dynamic Processing**: Analyze streaming data as it arrives, enabling immediate insights and actions.
- **Contextual Understanding**: Employ advanced algorithms to comprehend and interpret data contextually during the streaming process.
- **Middleware Support**: Implement middleware using Routerify to perform real-time data ingestion and intent detection.

### 🤖 Intelligent Agents with Tool Integration
- **Adaptive Agents**: Deploy agents that can interact with external tools and APIs based on real-time analysis.
- **Toolkits Integration**: Seamlessly integrate with calendars, weather services, databases, and more to enrich interactions and automate tasks.

### ⚡ Decision-Making Before Stream Completion
- **Proactive Responses**: Make informed decisions and take actions before the full stream is received, enhancing responsiveness.
- **Conditional Logic**: Implement complex logic paths that trigger based on specific data patterns or keywords detected during streaming.

### 🌐 Scalable Architecture
- **Distributed Processing**: Handle multiple concurrent streams efficiently, ensuring scalability as demand grows.
- **Fault Tolerance**: Maintain high availability and reliability through robust error handling and recovery mechanisms.

### 🔒 Secure Operations
- **Data Privacy**: Ensure all streamed and processed data is handled with the highest security standards.
- **Authentication & Authorization**: Implement strict access controls to protect sensitive information and system integrity.

### 🛠️ Open Router Integration
- **Routerify**: Leverage **Routerify**, an open-source routing library in Rust, to build a robust and efficient routing system.
- **Streaming Capabilities**: Utilize Routerify's middleware and routing features to manage real-time data streams effectively.
- **Extensibility**: Easily extend and customize routes and middleware to suit specific application needs, enabling seamless integration of additional functionalities.

---

## Architecture Overview

### 1. LLM Integration
- **Library Used**: Utilize a library like `rml_rtmp` for handling real-time streaming of LLM responses. While primarily designed for RTMP, it can be adapted for streaming text responses from an LLM.

### 2. Router Implementation
- **Routerify**: Employ **Routerify**, a robust routing system for HTTP requests in Rust, extended to handle streaming data. Routerify supports middleware, which can be leveraged for real-time data ingestion and processing.

    ```rust
    use routerify::{Router, Middleware, Request, Response, Body};
    use hyper::{StatusCode, body::HttpBody};
    use std::convert::Infallible;

    async fn stream_llm_response(req: Request<Body>) -> Result<Response<Body>, Infallible> {
        // Simulate streaming LLM response
        let (mut sender, body) = Body::channel();
        tokio::spawn(async move {
            for chunk in ["I've scheduled a meeting with John for next Wednesday at 10 AM.", "The weather forecast for that day is sunny with a high of 75°F."].iter() {
                if sender.send_data(chunk.as_bytes().to_vec()).await.is_err() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        });
        Ok(Response::new(body))
    }

    let router = Router::builder()
        .get("/chat", stream_llm_response)
        .build()
        .unwrap();
    ```

### 3. Real-Time Data Ingestion
- **Tinybird**: Although not directly a router, Tinybird can be utilized for real-time data ingestion and processing, handling high volumes of event streams, which is beneficial for managing data flow from the LLM and external tools.

### 4. Inflight Aggregation and Tool Integration
- **Middleware Implementation**: Develop middleware in Routerify to analyze streaming data from the LLM. This middleware can detect intents and initiate API calls to external services like calendar or weather APIs.

    ```rust
    async fn analyze_and_integrate(req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let (mut sender, body) = Body::channel();
        let mut response = req.into_body();
        while let Some(chunk) = response.data().await {
            let chunk = chunk.unwrap();
            let text = String::from_utf8_lossy(&chunk);
            // Analyze text for intents
            if text.contains("schedule a meeting") {
                // Call calendar API
            }
            if text.contains("check weather") {
                // Call weather API
            }
            sender.send_data(chunk).await.unwrap();
        }
        Ok(Response::new(body))
    }

    let router = Router::builder()
        .middleware(Middleware::pre(analyze_and_integrate))
        .build()
        .unwrap();
    ```

### 5. Intelligent Caching
- **Caching Mechanisms**: Implement caching within the middleware or as a separate service to store frequent queries or responses. This can be achieved using in-memory caching or integrating with external solutions like Redis.

### 6. ADBC Integration
- **Backend Service Interaction**: For fetching data from various backend services, implement custom logic or use libraries like `tokio-postgres` for database queries, or directly call APIs using `reqwest` or similar HTTP clients.

---

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

## Configuration Files

### 1. `.env`

This file holds environment variables for your application, such as database credentials and API keys.

```bash
# .env

# MidStream Configuration
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

## Installation Script (`install.sh`)

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

## Docker Configuration (`Dockerfile`)

A Docker setup for building and running the application.

```Dockerfile
# Dockerfile

# Stage 1: Build
FROM rust:1.71 as builder

WORKDIR /app

# Install system dependencies for MidStream
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

## Rust Project Configuration (`Cargo.toml`)

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
midstream = { git = "https://github.com/ruvnet/midstream.git", branch = "main" } # Assuming MidStream is available via Git
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

## Source Code

### 1. `src/main.rs`

The main entry point of the application. It loads environment variables, sets up the router, initializes MidStream, and starts the server.

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

    // Initialize MidStream
    let midstream = Midstream::new().await.expect("Failed to initialize MidStream");

    // Create the main router with MidStream instance
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

### 2. `src/router.rs`

Defines the application's routes and middleware using **Routerify**. Integrates **MidStream** for real-time data ingestion and analysis.

```rust
// src/router.rs

use routerify::{Router, Middleware, Request, Response, Body};
use hyper::StatusCode;
use std::convert::Infallible;
use crate::handlers::{stream_llm_response, fallback_handler};
use crate::midstream::Midstream;

pub fn create_router(midstream: Midstream) -> Router<Body, Infallible> {
    Router::builder()
        // Pre-middleware: analyze inflight data using MidStream
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

### 3. `src/handlers.rs`

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

### 4. `src/midstream.rs`

Implements **MidStream** integration for real-time data ingestion and inflight analysis. This module initializes MidStream and provides middleware for analyzing streaming data to detect intents and trigger tool integrations.

```rust
// src/midstream.rs

use midstream::MidstreamClient; // Hypothetical MidStream client
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
        // Initialize MidStream client with configurations from .env
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
    // In a real implementation, integrate with MidStream's data processing pipeline

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
                // e.g., schedule_meeting().await;
            }
            if text.contains("check weather") {
                // Trigger weather API call
                println!("[MIDSTREAM] Detected intent: check weather");
                // Implement actual API call here
                // e.g., check_weather().await;
            }

            // Optionally, store or process data with MidStream
            // Example: midstream_clone.client.lock().await.store_metric(...).await;
        }
    });

    // Pass the request through unchanged
    Ok(req)
}
```

> **Notes:**
> - The `MidstreamClient` and its methods are **hypothetical**. Replace them with actual implementations based on MidStream's API.
> - This middleware asynchronously processes the request body to detect intents and trigger corresponding actions.
> - In a real-world scenario, handle streaming data more gracefully, possibly integrating with MidStream's real-time data processing features.

 ## Acknowledgement

This project was inspired by and acknowledges the [HyprStream](https://github.com/yourusername/hyprstream) project, which provided foundational concepts for real-time data ingestion and processing. Special thanks to the HyprStream contributors for their valuable work.

---

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

## Implementation Considerations

### 1. **Streaming**
Ensure that LLM responses are streamed in real-time using asynchronous programming with `tokio`. This involves managing non-blocking I/O operations and handling data chunks as they arrive.

### 2. **Real-Time Analysis**
Utilize MidStream's capabilities to perform inflight data analysis, enabling the detection of actionable intents during the streaming process. This allows for immediate responses and tool integrations based on the content being streamed.

### 3. **Tool Integration**
Implement seamless integrations with external tools and APIs to enrich responses dynamically based on detected intents. For example, integrating with calendar APIs to schedule meetings or weather APIs to provide forecasts based on user queries.

### 4. **Intelligent Caching**
Leverage MidStream's caching mechanisms to store frequently accessed data, reducing latency for repeated queries. This ensures that common requests can be served rapidly without redundant processing.

### 5. **ADBC Integration**
Use MidStream's ADBC compatibility to interact with various backend datastores, ensuring data consistency and flexibility. This allows the application to connect with databases like PostgreSQL, Redis, or Snowflake seamlessly.

### 6. **Error Handling**
Implement robust error handling to manage failures in API calls, database interactions, or streaming processes. This ensures the application remains reliable and can recover gracefully from unexpected issues.

### 7. **Security**
Ensure secure communication with external APIs using HTTPS and proper authentication mechanisms. Protect sensitive data by securely managing environment variables and credentials, following best security practices.

### 8. **Scalability**
Design the application to handle multiple concurrent conversations efficiently, leveraging Rust's concurrency features and MidStream's scalable architecture. This ensures the application can grow with increasing demand without compromising performance.

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
