//! Failover and retry logic for provider fallbacks

use std::time::Duration;
use log::debug;

/// Retry policy for failed requests
#[derive(Debug, Clone)]
pub struct RetryPolicy
{   pub max_retries: usize
  , pub backoff_multiplier: f32
  , pub initial_backoff: Duration
}

impl RetryPolicy
{   /// Create a new retry policy
    pub fn new(
      max_retries: usize
    , backoff_multiplier: f32
    , initial_backoff_ms: u64
    ) -> Self
    {   RetryPolicy
        {   max_retries
          , backoff_multiplier
          , initial_backoff: Duration::from_millis(
              initial_backoff_ms
            )
        }
    }

    /// Calculate backoff duration for attempt number
    pub fn backoff_for_attempt(
      &self
    , attempt: usize
    ) -> Duration
    {   debug!("Calculating backoff for attempt {}", attempt);
        let multiplier 
          = self.backoff_multiplier.powi(attempt as i32);
        Duration::from_millis(
          (self.initial_backoff.as_millis() as f32 
            * multiplier) as u64
        )
    }
}

impl Default for RetryPolicy
{   fn default() -> Self
    {   RetryPolicy::new(3, 2.0, 100)
    }
}

/// Failover provider sequence
#[derive(Debug, Clone)]
pub struct FailoverSequence
{   pub providers: Vec<(crate::Provider, String)>
  , pub current_index: usize
}

impl FailoverSequence
{   /// Create a new failover sequence
    pub fn new(
      providers: Vec<(crate::Provider, String)>
    ) -> Self
    {   debug!(
          "Creating failover sequence with {} providers",
          providers.len()
        );
        FailoverSequence
        {   providers
          , current_index: 0
        }
    }

    /// Get the current provider
    pub fn current(&self) 
      -> Option<&(crate::Provider, String)>
    {   self.providers.get(self.current_index)
    }

    /// Move to the next provider
    pub fn next(&mut self) -> Option<&(crate::Provider, String)>
    {   self.current_index += 1;
        self.current()
    }

    /// Check if we have more providers to try
    pub fn has_next(&self) -> bool
    {   self.current_index + 1 < self.providers.len()
    }

    /// Reset to the first provider
    pub fn reset(&mut self)
    {   debug!("Resetting failover sequence");
        self.current_index = 0;
    }
}