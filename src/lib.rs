pub mod error;
pub mod config;
pub mod providers;
pub mod request;
pub mod failover;
pub mod client;
use serde::{Deserialize, Serialize};

/*

im making a new async-only rust library called allm  (All LLMs); 
where i will have one unique request syntax to all the public apis 
for LLM providers, and will have an automatic fail-over to switch 
to another provider when the api limits are reached.

allm/
├── Cargo.toml          # Main manifest
├── src/
│   ├── lib.rs          # Re-exports and main documentation
│   ├── error.rs        # Custom error types and handling
│   ├── config.rs       # Configuration for providers and failover
│   ├── client.rs       # Main client interface
│   ├── providers/      # Provider-specific implementations
│   │   ├── mod.rs      # Re-exports all providers
│   │   ├── openai.rs   # OpenAI API client
│   │   ├── anthropic.rs
│   │   ├── mistral.rs
│   │   └── ...         # Other providers
│   ├── request.rs      # Unified request/response types
│   ├── failover.rs     # Failover logic and retry policy
│   └── utils/          # Helper modules (e.g., rate limiting, logging)
│       ├── mod.rs
│       └── ...
├── examples/           # Example usage
├── tests/              # Integration and unit tests
└── benchmarks/         # Performance benchmarks (optional)

*/

/// ALLM API INTERFACE:

// ===== SendPrompt =====

pub type SendPromptReply = Result<String, crate::error::Error>;
pub type SendPromptReplySender 
  = tokio::sync::mpsc::UnboundedSender<SendPromptReply>;

pub struct SendPromptArgs 
{   pub prompt: String
  , pub model: String
  , pub reply: SendPromptReplySender
}

// ===== SetApiKeys =====

pub type SetApiKeysReply = Result<(), crate::error::Error>;
pub type SetApiKeysReplySender 
  = tokio::sync::mpsc::UnboundedSender<SetApiKeysReply>;

pub struct SetApiKeysArgs 
{   pub keys: Vec<ApiKeySpec>
  , pub reply: SetApiKeysReplySender
}

pub struct ApiKeySpec
{   pub provider: crate::Provider
  , pub model: String
  , pub key: String
}

// ===== GetModelLists =====

pub type GetModelListsReply 
  = Result<Vec<(crate::Provider, String)>, crate::error::Error>;
pub type GetModelListsReplySender 
  = tokio::sync::mpsc::UnboundedSender<GetModelListsReply>;

pub struct GetModelListsArgs 
{   pub reply: GetModelListsReplySender
}

// ===== KillProcess =====

pub type KillProcessReply = Result<(), crate::error::Error>;
pub type KillProcessReplySender 
  = tokio::sync::mpsc::UnboundedSender<KillProcessReply>;

pub struct KillProcessArgs 
{   pub reply: KillProcessReplySender
}

// ===== SetModelFallbackPreference =====

pub type SetModelFallbackPreferenceReply 
  = Result<(), crate::error::Error>;
pub type SetModelFallbackPreferenceSender 
  = tokio::sync::mpsc::UnboundedSender
    <SetModelFallbackPreferenceReply>;

pub struct SetModelFallbackPreferenceArgs 
{   pub preferences: Vec<(crate::Provider, String)>
  , pub reply: SetModelFallbackPreferenceSender
}

// ===== AllmHand (sender side) =====

pub struct AllmHand 
{   pub send_prompt_tx
      : tokio::sync::mpsc::UnboundedSender<SendPromptArgs>
  , pub set_api_keys_tx
      : tokio::sync::mpsc::UnboundedSender<SetApiKeysArgs>
  , pub get_model_lists_tx
      : tokio::sync::mpsc::UnboundedSender<GetModelListsArgs>
  , pub kill_process_tx
      : tokio::sync::mpsc::UnboundedSender<KillProcessArgs>
  , pub set_model_fallback_preference_tx
      : tokio::sync::mpsc::UnboundedSender
        <SetModelFallbackPreferenceArgs>
}

// ===== AllmFoot (receiver side) =====

pub struct AllmFoot 
{   pub send_prompt_rx
      : tokio::sync::mpsc::UnboundedReceiver<SendPromptArgs>
  , pub set_api_keys_rx
      : tokio::sync::mpsc::UnboundedReceiver<SetApiKeysArgs>
  , pub get_model_lists_rx
      : tokio::sync::mpsc::UnboundedReceiver<GetModelListsArgs>
  , pub kill_process_rx
      : tokio::sync::mpsc::UnboundedReceiver<KillProcessArgs>
  , pub set_model_fallback_preference_rx
      : tokio::sync::mpsc::UnboundedReceiver
        <SetModelFallbackPreferenceArgs>
}

/// ALLM STRUCTURES:

/// Enum representing all targeted supported LLM providers.
/// Each variant corresponds to a public API or platform.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum Provider 
{
  // ===== CORE PROVIDERS =====
  /// Mistral AI (Le Chat, Mistral models)
  MistralAi
  ,
  /// OpenAI (ChatGPT, GPT-4, etc.)
  OpenAI
  ,
  /// Anthropic (Claude models)
  Anthropic
  ,
  /// Google (AI Studio: Gemma, Gemini)
  Google
  ,
  /// Meta (Llama, etc., typically self-hosted)
  Meta
  ,
  /// Perplexity AI (Perplexity models and API)
  PerplexityAi
  ,
  /// xAI (Grok model)
  Xai
  ,
  /// AI21 Studio (Jamba, Jurassic models)
  Ai21Studio
  ,
  /// Alibaba Cloud (Qwen, Tongyi Qianwen models)
  Alibaba
  ,
  // ===== COMBINED/AGGREGATOR PROVIDERS =====
  /// Hugging Face Inference API
  HuggingFaceInterface
  ,
  /// Groq (owned by Nvidia; hosts Llama 3, Gemma, DeepSeek)
  Groq
  ,
  /// Cloudflare Workers AI (serverless inference)
  CloudflareAi
  ,
  /// Together AI (hosts Llama, DeepSeek, Mixtral)
  TogetherAi
  ,
  /// Cerebras (high-performance inference)
  Cerebras
  ,
  /// OpenRouter (unified API over many providers)
  OpenRouter
  ,
  /// Fireworks AI (hosts Llama, Mixtral)
  FireworksAi
  ,
  /// Replicate (hosts open-source models)
  Replicate
  ,
  // ===== SELF-HOSTED/LOCAL =====
  /// Local/self-hosted models (Ollama, LM Studio, vLLM)
  Local
}

/// Information about a model's capabilities and limits.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo 
{   /// Human-readable name (e.g., "mistral-tiny", "gpt-4")
    pub name: String
  , /// Maximum context window (in tokens)
    pub max_context_tokens: usize
  , /// Maximum tokens the model can generate in response
    pub max_response_tokens: usize
  , /// Whether the model supports saving context between calls
    pub can_save_context: bool
  , /// Input modalities supported by the model
    pub input_modalities: ModelModalities
  , /// Whether the model supports streaming responses
    pub supports_streaming: bool
  , /// Whether the model supports function/tool calling
    pub supports_tools: bool
  , /// Provider of the model
    pub provider: crate::Provider
  , /// Default system prompt or behavior instructions
    pub default_system_prompt: Option<String>
  , /// List of file extensions supported for file input
    pub supported_file_extensions: Option<Vec<String>>
  , /// Cost per 1M input tokens (in USD)
    pub cost_per_million_input_tokens: Option<f32>
  , /// Cost per 1M output tokens (in USD)
    pub cost_per_million_output_tokens: Option<f32>
  , /// Whether the model is currently available
    pub is_available: bool
}

/// Represents a single input modality
#[derive(Debug, Clone, PartialEq)]
pub enum BaseModality 
{   Text
  , Image
  , Video
  , File
}

/// Represents a combination of modalities (e.g., Text + Image)
#[derive(Debug, Clone, PartialEq)]
pub struct CombinedModality 
{   pub modalities: Vec<BaseModality>
}

/// Represents a single or combined input modality
#[derive(Debug, Clone, PartialEq)]
pub enum InputModality 
{   Single(BaseModality)
  , Combined(CombinedModality)
}

/// All possible input modalities a model supports
#[derive(Debug, Clone, PartialEq)]
pub struct ModelModalities 
{   pub supported: Vec<InputModality>
}