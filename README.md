# allm - All LLMs

Single unified frontend for all free LLM APIs.

Tired of hitting free-tier limits on different LLM providers? **ALLM** provides:

1. **Unified API** - One interface for all LLM providers (Mistral, OpenAI, Anthropic, etc.)
2. **Automatic Failover** - Seamlessly switch providers/models when rate limits are hit
3. **Multi-Model Adjudication** - Query multiple models simultaneously and synthesize optimal responses

This is a **Rust async library** built on an efficient actor pattern with zero-copy message passing.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [API Overview](#api-overview)
- [Actor Pattern Design](#actor-pattern-design)
- [API Key Management](#api-key-management)
- [File Structure](#file-structure)
- [Event Loop Design](#event-loop-design)
- [Testing](#testing)

---

## Quick Start

### Basic Usage

```rust
use allm::AllmBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create backend with Mistral API key
    let backend = AllmBackend::new(
        std::env::var("MISTRAL_API_KEY").ok()
    );

    // Send a prompt (returns immediately)
    let reply_rx = backend.send_prompt(
        "What is Rust?".to_string(),
        "mistral-small-latest".to_string()
    ).await?;

    // Wait for response (with timeout)
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        reply_rx.recv()
    ).await {
        Ok(Some(Ok(response))) => println!("Response: {}", response),
        Ok(Some(Err(e))) => println!("Error: {}", e),
        _ => println!("Timeout or channel closed"),
    }

    // Graceful shutdown
    backend.shutdown().await?;
    Ok(())
}
```

### Set API Keys

```rust
// Master key (fallback for all models)
backend.set_api_keys(vec![
    ApiKeySpec {
        provider: Provider::MistralAi,
        model: String::new(),  // Empty = master
        key: "your-api-key".to_string(),
    }
]).await?;

// Or model-specific key
backend.set_api_keys(vec![
    ApiKeySpec {
        provider: Provider::MistralAi,
        model: "mistral-large".to_string(),
        key: "different-key".to_string(),
    }
]).await?;
```

---

## Architecture

ALLM uses an **actor pattern** where each component is an independent async actor:

```
┌─────────────────────────────────────────────────────────────┐
│  User Code                                                  │
│  backend.send_prompt(prompt, model)  ← returns immediately │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
        ┌────────────────────┐
        │  AllmBackend Actor │
        │  - Routes commands │
        │  - Manages state   │
        └────────┬───────────┘
                 │
        ┌────────┴──────────┬─────────────────────┐
        │                   │                     │
        ▼                   ▼                     ▼
   ┌──────────┐       ┌──────────┐         ┌──────────┐
   │ Mistral  │       │  OpenAI  │  ...    │Anthropic │
   │ Client   │       │ Client   │         │ Client   │
   └────┬─────┘       └────┬─────┘         └────┬─────┘
        │                  │                    │
        ▼                  ▼                    ▼
   HTTP Req          HTTP Req              HTTP Req
        │                  │                    │
        ▼                  ▼                    ▼
   Mistral API      OpenAI API           Anthropic API
        │                  │                    │
        └──────────────────┼────────────────────┘
                           ▼
                    Results → reply_tx
                           │
                    User receives on reply_rx
```

### Key Components

#### 1. AllmBackend (orchestrator)
- Owns all provider clients
- Routes commands based on current model/provider
- Manages API keys and fallback preferences
- Task is owned and auto-managed by struct

#### 2. Provider Clients (e.g., MistralClient)
- Independent actor for each provider
- Handles all API communication
- Queues requests internally
- Processes async operations in background

#### 3. Command Routing
Each select! arm in the event loop routes to the right provider:
```rust
match state.current_model.0 {
  Provider::MistralAi => {
    state.mistral_client.send_prompt(
      cmd.prompt, cmd.model, cmd.reply
    ).await?;
  }
  Provider::OpenAI => { /* ... */ }
}
```

---

## API Overview

### AllmBackend Methods

```rust
// Create backend
let backend = AllmBackend::new(api_key);

// Send prompt (returns immediately with reply receiver)
let reply_rx = backend.send_prompt(prompt, model).await?;
let result = reply_rx.recv().await;

// Set API keys (master or model-specific)
backend.set_api_keys(vec![ApiKeySpec { ... }]).await?;

// Get available models
let reply_rx = backend.get_model_lists().await?;
let models = reply_rx.recv().await;

// Set fallback preferences
backend.set_model_fallback_preference(vec![
    (Provider::MistralAi, "mistral-small".to_string()),
    (Provider::MistralAi, "mistral-large".to_string()),
]).await?;

// Graceful shutdown
backend.shutdown().await?;
```

### MistralClient Methods

```rust
// Create client
let client = MistralClient::new(api_key, error_tx);

// Send prompt directly (pass reply sender)
client.send_prompt(prompt, model, reply_tx).await?;

// Get models
client.get_available_models(reply_tx).await?;

// Set API key (master or model-specific)
client.set_api_key(Some(model), key, reply_tx).await?;

// Shutdown
client.shutdown().await?;
```

---

## Actor Pattern Design

Every actor in ALLM follows the same pattern:

```rust
pub struct MyActor {
  tx: mpsc::UnboundedSender<Command>,
  _task: tokio::task::JoinHandle<()>,  // Owned
}

impl MyActor {
  // 1. Spawn task with receiver
  pub fn new(config) -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    let _task = tokio::spawn(async move {
      event_loop(rx, config).await;
    });
    MyActor { tx, _task }
  }

  // 2. Queue command (returns immediately)
  pub async fn send_command(&self, cmd) -> Result<(), Error> {
    self.tx.send(cmd)?;
    Ok(())
  }
}

// 3. Event loop only routes, never blocks
async fn event_loop(mut rx, config) {
  loop {
    match rx.recv().await {
      Some(Command::X) => { /* handle */ }
      Some(Command::Y) => { /* handle */ }
      None => break,
    }
  }
}
```

**Key Benefits:**
- Task is owned by struct → automatic cleanup on drop
- Public methods return near-instantly
- Long operations happen in background
- Composable: actors can call other actors

---

## API Key Management

### Two-Tier System

Each provider supports **master key + model-specific overrides**:

```
Lookup order:
  1. Check model-specific key: model_keys.get("mistral-large")
  2. Fall back to master key: master_key
  3. Error if neither exist
```

### Example

```rust
// Set master key (for free-tier models)
backend.set_api_keys(vec![
    ApiKeySpec {
        provider: Provider::MistralAi,
        model: "".to_string(),  // Empty string = master
        key: "free-tier-key".to_string(),
    }
]).await?;

// Set paid model key (overrides master)
backend.set_api_keys(vec![
    ApiKeySpec {
        provider: Provider::MistralAi,
        model: "mistral-large".to_string(),
        key: "paid-tier-key".to_string(),
    }
]).await?;

// Now:
// - mistral-small-latest → uses free-tier-key
// - mistral-large → uses paid-tier-key
```

---

## File Structure

```
allm/
├── Cargo.toml                      # Dependencies
├── src/
│   ├── lib.rs                      # Main exports
│   ├── error.rs                    # Error types (Clone + Eq)
│   ├── config.rs                   # Configuration
│   ├── client.rs                   # AllmBackend actor
│   ├── request.rs                  # Unified types
│   ├── failover.rs                 # Retry/failover logic
│   └── providers/
│       ├── mod.rs                  # Provider exports
│       └── mistral.rs              # Mistral AI actor
├── tests/
│   ├── integration_tests.rs        # Integration tests
│   └── providers.json              # Test config
├── examples/
│   └── basic.rs                    # Basic example
└── README.md
```

### Module Breakdown

| Module | Responsibility |
|--------|-----------------|
| `lib.rs` | Re-exports all public types |
| `error.rs` | Unified error type (`Clone + Eq`) |
| `config.rs` | Provider/failover config structs |
| `client.rs` | `AllmBackend` actor + state |
| `request.rs` | Unified request/response types |
| `failover.rs` | Retry policy & failover sequence |
| `providers/mistral.rs` | `MistralClient` actor |

---

## Event Loop Design

### Critical: No Awaits in tokio::select!

The select! loop is ONLY for routing commands—it never blocks:

```rust
loop {
  tokio::select! {
    // Fast routing only
    Some(cmd) = send_prompt_rx.recv() => {
      match state.current_model.0 {
        Provider::MistralAi => {
          // ONE-LINER: pass reply directly to provider
          let _ = state.mistral_client
            .send_prompt(cmd.prompt, cmd.model, cmd.reply)
            .await;  // Returns immediately
        }
      }
      // Loop continues immediately
    }

    Some(cmd) = set_api_keys_rx.recv() => {
      // Update state, return immediately
      for key_spec in cmd.keys {
        state.api_keys.insert((key_spec.provider, key_spec.model), key_spec.key);
      }
      let _ = cmd.reply.send(Ok(()));
    }
  }
}
```

### Flow for send_prompt

```
1. User calls: backend.send_prompt(prompt, model)
   ↓ Returns immediately with reply_rx

2. Command queued to send_prompt_rx

3. Select! arm receives (doesn't block others)

4. Route to provider:
   mistral_client.send_prompt(prompt, model, reply_tx)
   ↓ Returns immediately, queues internally

5. Select! loop continues processing more commands

6. Mistral actor processes async, sends result to reply_tx

7. User receives on their reply_rx
```

### Why This Design

1. **Non-blocking select!** - Can handle thousands of concurrent requests
2. **No spawning overhead** - Providers queue internally
3. **Simple routing** - One match per command type
4. **Composable** - Same pattern at every level (Backend → Provider → HTTP)

---

## Testing

### Unit Tests (always run)

```bash
cargo test
```

Tests:
- Client initialization
- API key management
- Error handling

### Integration Tests (require API keys)

```bash
export MISTRAL_API_KEY="your-key"
cargo test -- --ignored --nocapture
```

Tests:
- Mistral API communication
- Multi-model scenarios
- Failover behavior

### Test Configuration

Edit `tests/providers.json`:

```json
{
  "providers": [
    {
      "name": "mistral",
      "main_key": "MISTRAL_API_KEY",
      "models": [
        {
          "model_name": "mistral-small",
          "model_key": "MISTRAL_SMALL_KEY"
        },
        {
          "model_name": "mistral-large",
          "model_key": "MISTRAL_LARGE_KEY"
        }
      ]
    }
  ]
}
```

---

## Provider API Pattern

All providers (Mistral, OpenAI, Anthropic, etc.) follow the same interface:

```rust
pub struct ProviderClient {
  tx: mpsc::UnboundedSender<ProviderCommand>,
  _task: tokio::task::JoinHandle<()>,
}

impl ProviderClient {
  pub fn new(api_key, error_tx) -> Self { /* ... */ }

  // All methods take the reply sender directly
  pub async fn send_prompt(
    &self,
    prompt: String,
    model: String,
    reply: mpsc::UnboundedSender<Result<String, Error>>,
  ) -> Result<(), Error> {
    self.tx.send(ProviderCommand::SendPrompt {
      prompt, model, reply
    })
  }

  pub async fn get_models(
    &self,
    reply: mpsc::UnboundedSender<Result<Vec<String>, Error>>,
  ) -> Result<(), Error> { /* ... */ }

  pub async fn set_api_key(
    &self,
    model: Option<String>,
    key: String,
    reply: mpsc::UnboundedSender<Result<(), Error>>,
  ) -> Result<(), Error> { /* ... */ }
}
```

This allows:
- **Direct reply forwarding** - No spawning overhead
- **Consistent API** - Same interface for all providers
- **Composable routing** - AllmBackend just calls the right provider
- **Zero-copy** - Reply senders pass through without cloning

---

## REST API Endpoints

### Mistral
- **Chat:** `POST https://api.mistral.ai/v1/chat/completions`
- **Models:** `GET https://api.mistral.ai/v1/models`

Request format:
```json
{
  "model": "mistral-small-latest",
  "messages": [
    { "role": "user", "content": "..." }
  ],
  "max_tokens": 1024,
  "temperature": 0.7
}
```

---

## License

MIT

---

## Author

Created by SEE (@seeya)