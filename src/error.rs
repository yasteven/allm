use std::fmt;

/// Custom error type for ALLM operations
/// Implements Clone for sending through channels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error
{   /// API key is missing for a provider
    MissingApiKey(String)
  , /// Provider not yet implemented
    ProviderNotImplemented(String)
  , /// HTTP request error
    HttpError(String)
  , /// API returned an error response
    ApiError(String)
  , /// Failed to parse API response
    ParseError(String)
  , /// No choices in API response
    NoChoicesInResponse
  , /// Prompt not found in queue
    PromptNotFound(usize)
  , /// Rate limit exceeded
    RateLimitExceeded
  , /// Context window exceeded
    ContextWindowExceeded
  , /// Invalid configuration
    InvalidConfiguration(String)
  , /// Timeout error
    Timeout
  , /// Generic error
    Other(String)
}

impl fmt::Display for Error
{   fn fmt(&self, f: &mut fmt::Formatter<'_>) 
      -> fmt::Result
    {   match self
        {   Error::MissingApiKey(provider) => {
              write!(f, "Missing API key for: {}", provider)
            }
          , Error::ProviderNotImplemented(provider) => {
              write!(f, 
                "Provider not yet implemented: {}", 
                provider
              )
            }
          , Error::HttpError(msg) => {
              write!(f, "HTTP error: {}", msg)
            }
          , Error::ApiError(msg) => {
              write!(f, "API error: {}", msg)
            }
          , Error::ParseError(msg) => {
              write!(f, "Parse error: {}", msg)
            }
          , Error::NoChoicesInResponse => {
              write!(f, "API response contained no choices")
            }
          , Error::PromptNotFound(id) => {
              write!(f, "Prompt not found in queue: {}", id)
            }
          , Error::RateLimitExceeded => {
              write!(f, "API rate limit exceeded")
            }
          , Error::ContextWindowExceeded => {
              write!(f, 
                "Request exceeds model context window"
              )
            }
          , Error::InvalidConfiguration(msg) => {
              write!(f, "Invalid configuration: {}", msg)
            }
          , Error::Timeout => {
              write!(f, "Request timed out")
            }
          , Error::Other(msg) => {
              write!(f, "Error: {}", msg)
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<String> for Error
{   fn from(s: String) -> Self
    {   Error::Other(s)
    }
}

impl From<&str> for Error
{   fn from(s: &str) -> Self
    {   Error::Other(s.to_string())
    }
}