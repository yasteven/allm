//! Configuration for ALLM providers and failover behavior

use serde::{Deserialize, Serialize};

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig
{   /// Provider name
    pub name: String
  , /// API base URL (if custom)
    pub api_base: Option<String>
  , /// Request timeout in seconds
    pub timeout_secs: Option<u64>
  , /// Enable detailed logging
    pub verbose: Option<bool>
}

/// Failover configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig
{   /// Enable automatic failover
    pub enabled: bool
  , /// Max retry attempts per provider
    pub max_retries: usize
  , /// Backoff multiplier for retries
    pub backoff_multiplier: f32
  , /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64
}

impl Default for FailoverConfig
{   fn default() -> Self
    {   FailoverConfig
        {   enabled: true
          , max_retries: 3
          , backoff_multiplier: 2.0
          , initial_backoff_ms: 100
        }
    }
}

/// ALLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllmConfig
{   /// Provider configurations
    pub providers: Vec<ProviderConfig>
  , /// Failover configuration
    pub failover: FailoverConfig
}

impl Default for AllmConfig
{   fn default() -> Self
    {   AllmConfig
        {   providers: vec![]
          , failover: FailoverConfig::default()
        }
    }
}