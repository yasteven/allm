// allm/tests/integration_tests.rs

use allm::{AllmBackend, ApiKeySpec, Provider};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestConfig
{ providers: Vec<ProviderTestConfig>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderTestConfig
{ name: String
  , main_key: String     // actual key or placeholder
  , models: Vec<ModelTestConfig>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelTestConfig
{ model_name: String
  , model_key: String    // actual key or placeholder
}

/// Load providers.json from tests/ directory
fn load_test_config() -> TestConfig
{ let path = "tests/providers.json";
  let content = fs::read_to_string(path)
    .expect("Failed to read tests/providers.json – make sure it exists!");
  serde_json::from_str(&content)
    .expect("Invalid JSON in tests/providers.json")
}


/// Find a provider config by name
fn find_provider_config<'a>(config: &'a TestConfig, name: &str)
  -> Option<&'a ProviderTestConfig>
{ config.providers.iter().find(|p| p.name == name)
}

/// Map provider name from JSON to Provider enum
fn provider_from_name(name: &str) -> Option<Provider>
{ match name
  { "mistral"    => Some(Provider::MistralAi)
  , "openai"     => Some(Provider::OpenAI)
  , "anthropic"  => Some(Provider::Anthropic)
  , _            => None
  }
}

#[tokio::test]
async fn test_backend_with_mistral_integration()
{ let _ = env_logger::builder()
      .is_test(true)
      .try_init();

  let config = load_test_config();

  let mistral_config = match find_provider_config(&config, "mistral")
  { Some(c) => c
  , None    => panic!("No 'mistral' provider found in tests/providers.json")
  };

  let provider = provider_from_name(&mistral_config.name)
    .expect("Unknown provider name in config");

  log::info!("provider = {:?}", provider);

  let mut api_keys = Vec::new();

  // Master key
  let mkey = mistral_config.main_key.clone();
  { println!("Using master key from config");
    api_keys.push(ApiKeySpec
    { provider // THIS MOVES PROIDER
    , model: String::new()   // empty = master key
    , key : mkey
    })
  }

  // Per-model keys
  for model_cfg in &mistral_config.models
  { let key = model_cfg.model_key.clone();
    { println!
      ( "Using key for model '{}'"
      , model_cfg.model_name
      );
      api_keys.push(ApiKeySpec
      { provider : Provider::MistralAi // KEEP THE HARDCODED MISTRAL ! (we're in mistral_config, dummy!, provider was moved above, do not remove this ever again!)
      , model: model_cfg.model_name.clone()
      , key
      })
    }
  }

  if api_keys.is_empty()
  { eprintln!("No usable Mistral API keys found in providers.json!");
    panic!("No API keys available for integration test")
  }

  println!("Collected {} usable API key(s)", api_keys.len());

  log::trace!(" // Create backend ...");
  let backend = AllmBackend::new(None);

  log::trace!(" // Register keys ...");
  let set_res = backend.set_api_keys(api_keys).await
    .expect("Failed to queue set_api_keys");
  let mut set_rx = set_res;

  let set_result = timeout(Duration::from_secs(5), set_rx.recv())
    .await
    .expect("Timeout waiting for set_api_keys reply")
    .expect("set_api_keys channel closed");

  assert!(set_result.is_ok(), "set_api_keys failed: {:?}", set_result.err());
  println!("API keys successfully registered with backend");

  // Choose model to test
  let test_model = mistral_config
    .models
    .first()
    .map(|m| m.model_name.clone())
    .unwrap_or_else(|| "mistral-small-latest".to_string());

  println!("Sending prompt to model: {}", test_model);

  let send_res = backend
    .send_prompt
    ( "Say 'TEST SUCCESSFUL' in all caps and nothing else.".to_string()
    , test_model
    )
    .await
    .expect("Failed to queue send_prompt");

  let mut response_rx = send_res;

  let result = timeout(Duration::from_secs(30), response_rx.recv())
    .await
    .expect("Timeout waiting for response")
    .expect("Response channel closed");

  match result
  { Ok(text) =>
    { let trimmed = text.trim();
      println!("Response ({} chars): {}", trimmed.len(), trimmed);
      assert!(!trimmed.is_empty(), "Empty response received");
      if trimmed.contains("TEST SUCCESSFUL")
      { println!("Integration test passed perfectly!")
      }
    }
  , Err(e) => panic!("Request failed: {}", e)
  }

  backend.shutdown().await.expect("Failed to shutdown backend");
  println!("Backend shut down cleanly");
  //panic!("Look at the output!");
}

#[tokio::test]
async fn test_backend_initialization_and_shutdown()
{ let _ = env_logger::builder()
      .is_test(true)
      .try_init();

  let backend = AllmBackend::new(None);
  println!("Backend initialized successfully");

  backend.shutdown().await.expect("Backend shutdown failed");
  println!("Backend shut down cleanly")
}