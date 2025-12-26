use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Test configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig
{   pub providers: Vec<ProviderConfig>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig
{   pub name: String
  , pub main_key: String
  , pub models: Vec<ModelConfig>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig
{   pub model_name: String
  , pub model_key: String
}

/// Load test configuration from JSON file
fn load_test_config(path: &str) 
  -> Result<TestConfig, Box<dyn std::error::Error>>
{   let config_str = fs::read_to_string(path)?;
    let config: TestConfig = serde_json::from_str(&config_str)?;
    Ok(config)
}

/// Get API key from environment or config
fn get_api_key(env_var: &str) 
  -> Result<String, Box<dyn std::error::Error>>
{   std::env::var(env_var)
      .map_err(|_| {
        format!("Environment variable {} not set", env_var)
          .into()
      })
}

#[tokio::test]
async fn test_mistral_client_creation()
{   let client = allm::providers::mistral::MistralClient::new(
      Some("test-key".to_string())
    );
    assert!(client.api_key.is_some());
}

#[tokio::test]
async fn test_mistral_set_api_key()
{   let mut client = allm::providers::mistral::MistralClient::new(
      None
    );
    assert!(client.api_key.is_none());
    
    let _ = client.set_api_key("new-key".to_string()).await;
    assert!(client.api_key.is_some());
}

#[tokio::test]
#[ignore]
async fn test_mistral_get_models()
{   // Load test config
    let config = match load_test_config(
      "tests/providers.json"
    ) {
      Ok(c) => c,
      Err(e) => {
        println!("Warning: Failed to load config: {}", e);
        return;
      }
    };

    // Find Mistral config
    let mistral_config = config.providers
      .iter()
      .find(|p| p.name == "mistral");

    if let Some(provider) = mistral_config
    {   // Try to get API key
        match get_api_key(&provider.main_key)
        {   Ok(api_key) => {
              let client = allm::providers::mistral::MistralClient::new(
                Some(api_key)
              );
              
              match client.get_available_models().await
              {   Ok(models) => {
                    println!("Available Mistral models:");
                    for model in models
                    {   println!("  - {}", model);
                    }
                    assert!(!models.is_empty());
                  }
                , Err(e) => {
                    println!("Failed to get models: {}", e);
                  }
              }
            }
          , Err(_) => {
              println!(
                "Skipping test: {} not set in environment",
                provider.main_key
              );
            }
        }
    } else
    {   println!("Mistral config not found in providers.json");
    }
}

#[tokio::test]
#[ignore]
async fn test_mistral_send_prompt()
{   // Load test config
    let config = match load_test_config(
      "tests/providers.json"
    ) {
      Ok(c) => c,
      Err(e) => {
        println!("Warning: Failed to load config: {}", e);
        return;
      }
    };

    // Find Mistral config
    let mistral_config = config.providers
      .iter()
      .find(|p| p.name == "mistral");

    if let Some(provider) = mistral_config
    {   // Get API key
        match get_api_key(&provider.main_key)
        {   Ok(api_key) => {
              let client = allm::providers::mistral::MistralClient::new(
                Some(api_key)
              );
              
              // Use first available model
              if let Some(model_config) = provider.models.first()
              {   let model_name = &model_config.model_name;
                  
                  match client
                    .send_prompt("Say hello", model_name)
                    .await
                  {   Ok(response) => {
                        println!(
                          "Response from {}: {}",
                          model_name, response
                        );
                        assert!(
                          !response.is_empty(),
                          "Response should not be empty"
                        );
                      }
                    , Err(e) => {
                        println!(
                          "Failed to send prompt: {}", e
                        );
                      }
                  }
              }
            }
          , Err(_) => {
              println!(
                "Skipping test: {} not set",
                provider.main_key
              );
            }
        }
    } else
    {   println!("Mistral config not found");
    }
}

#[tokio::test]
#[ignore]
async fn test_mistral_per_model_keys()
{   // Load test config
    let config = match load_test_config(
      "tests/providers.json"
    ) {
      Ok(c) => c,
      Err(e) => {
        println!("Warning: Failed to load config: {}", e);
        return;
      }
    };

    // Find Mistral config
    let mistral_config = config.providers
      .iter()
      .find(|p| p.name == "mistral");

    if let Some(provider) = mistral_config
    {   println!("Testing per-model API keys for Mistral");
        
        // Test each model's specific key
        for model_config in &provider.models
        {   match get_api_key(&model_config.model_key)
            {   Ok(api_key) => {
                  let client = allm::providers::mistral::MistralClient::new(
                    Some(api_key)
                  );
                  
                  match client
                    .send_prompt("Test", &model_config.model_name)
                    .await
                  {   Ok(response) => {
                        println!(
                          "✓ Model {} responded: {}",
                          model_config.model_name,
                          &response[..50.min(response.len())]
                        );
                      }
                    , Err(e) => {
                        println!(
                          "✗ Model {} failed: {}",
                          model_config.model_name, e
                        );
                      }
                  }
                }
              , Err(_) => {
                  println!(
                    "⊘ Skipping {}: {} not set",
                    model_config.model_name,
                    model_config.model_key
                  );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_backend_initialization()
{   let backend = allm::AllmBackend::new(None);
    println!("Backend created successfully");
    
    // Just verify it doesn't panic
    let _ = backend.shutdown().await;
}

#[tokio::test]
#[ignore]
async fn test_backend_set_api_keys()
{   let backend = allm::AllmBackend::new(None);
    
    let keys = vec![
      allm::ApiKeySpec
      {   provider: allm::Provider::MistralAi
        , model: "mistral-small".to_string()
        , key: std::env::var("MISTRAL_API_KEY")
            .unwrap_or_else(|_| "test-key".to_string())
      }
    ];

    let reply_rx = backend.set_api_keys(keys).await;
    assert!(reply_rx.is_ok());

    let mut rx = reply_rx.unwrap();
    if let Some(result) = rx.recv().await
    {   match result
        {   Ok(_) => println!("API keys set successfully"),
          , Err(e) => println!("Error: {}", e)
        }
    }

    let _ = backend.shutdown().await;
}

#[tokio::test]
#[ignore]
async fn test_backend_get_models()
{   let backend = allm::AllmBackend::new(
      std::env::var("MISTRAL_API_KEY").ok()
    );
    
    let reply_rx = backend.get_model_lists().await;
    assert!(reply_rx.is_ok());

    let mut rx = reply_rx.unwrap();
    if let Some(result) = rx.recv().await
    {   match result
        {   Ok(models) => {
              println!("Retrieved {} models", models.len());
              assert!(!models.is_empty());
            }
          , Err(e) => {
              println!("Error fetching models: {}", e);
            }
        }
    }

    let _ = backend.shutdown().await;
}

#[tokio::test]
#[ignore]
async fn test_backend_send_prompt()
{   let mistral_key = std::env::var("MISTRAL_API_KEY")
      .ok();
    
    if mistral_key.is_none()
    {   println!("Skipping: MISTRAL_API_KEY not set");
        return;
    }

    let backend = allm::AllmBackend::new(mistral_key);
    
    let reply_rx = backend
      .send_prompt(
        "What is 2+2?".to_string(),
        "mistral-small".to_string()
      )
      .await;
    
    assert!(reply_rx.is_ok());

    let mut rx = reply_rx.unwrap();
    match tokio::time::timeout(
      std::time::Duration::from_secs(15),
      rx.recv()
    ).await
    {   Ok(Some(result)) => {
          match result
          {   Ok(response) => {
                println!("Response: {}", response);
                assert!(!response.is_empty());
              }
            , Err(e) => {
                println!("API Error: {}", e);
              }
          }
        }
      , Ok(None) => {
          println!("Channel closed");
        }
      , Err(_) => {
          println!("Timeout waiting for response");
        }
    }

    let _ = backend.shutdown().await;
}