//! Unified request and response types for ALLM

use serde::{Deserialize, Serialize};

/// Unified prompt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRequest
{   /// The prompt text
    pub prompt: String
  , /// Provider to use
    pub provider: crate::Provider
  , /// Model name
    pub model: String
  , /// Optional system message
    pub system_message: Option<String>
  , /// Max tokens to generate
    pub max_tokens: Option<usize>
  , /// Temperature for sampling
    pub temperature: Option<f32>
}

/// Unified prompt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptResponse
{   /// Generated text
    pub text: String
  , /// Provider that generated it
    pub provider: crate::Provider
  , /// Model that generated it
    pub model: String
  , /// Tokens used
    pub tokens_used: Option<usize>
}

/// Unified error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse
{   /// Error code
    pub code: String
  , /// Error message
    pub message: String
  , /// Provider that errored
    pub provider: crate::Provider
}