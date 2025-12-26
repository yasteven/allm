use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use log::{debug, trace, error, info};
use std::collections::HashMap;

const MISTRAL_API_BASE: &str 
  = "https://api.mistral.ai/v1";

// ===== Message Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage
{   pub role: String
  , pub content: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralChatRequest
{   pub model: String
  , pub messages: Vec<ChatMessage>
  , #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>
  , #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>
  , #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>
}

#[derive(Debug, Clone, Deserialize)]
pub struct MistralChatResponse
{   pub choices: Vec<Choice>
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice
{   pub message: ChatMessage
  , pub finish_reason: Option<String>
}

#[derive(Debug, Clone, Deserialize)]
pub struct MistralModelsResponse
{   pub data: Vec<ModelData>
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelData
{   pub id: String
  , #[serde(default)]
    pub owned_by: Option<String>
}

// ===== Mistral Client Actor =====

/// Commands for MistralClient actor
pub enum MistralCommand
{   SendPrompt
    {   prompt: String
      , model: String
      , reply: mpsc::UnboundedSender<crate::SendPromptReply>
    }
  , GetModels
    {   reply: mpsc::UnboundedSender
        <Result<Vec<String>, crate::error::Error>>
    }
  , SetApiKey
    {   model: Option<String>
      , key: String
      , reply: mpsc::UnboundedSender
        <Result<(), crate::error::Error>>
    }
  , Shutdown
}

/// Mistral client state
pub struct MistralClientState
{   master_key: Option<String>
  , model_keys: HashMap<String, String>
  , http_client: reqwest::Client
}

impl MistralClientState
{   pub fn new(master_key: Option<String>) -> Self
    {   debug!("Creating MistralClientState");
        MistralClientState
        {   master_key
          , model_keys: HashMap::new()
          , http_client: reqwest::Client::new()
        }
    }

    fn get_api_key(&self, model: &str) 
      -> Result<String, crate::error::Error>
    {   if let Some(key) = self.model_keys.get(model)
        {   debug!("Using model-specific key for: {}", model);
            return Ok(key.clone());
        }
        
        if let Some(key) = &self.master_key
        {   debug!(
              "Using master key for model: {}", 
              model
            );
            return Ok(key.clone());
        }

        error!("No API key for model: {}", model);
        Err(crate::error::Error::MissingApiKey(
          format!("Mistral:{}", model)
        ))
    }

    fn set_master_key(&mut self, key: String)
    {   debug!("Setting master key");
        self.master_key = Some(key);
    }

    fn set_model_key(&mut self, model: String, key: String)
    {   debug!("Setting model key for: {}", model);
        self.model_keys.insert(model, key);
    }

    async fn handle_send_prompt(
      &self
    , prompt: String
    , model: String
    ) -> Result<String, crate::error::Error>
    {   debug!("Handling send_prompt for: {}", model);
        
        let api_key = self.get_api_key(&model)?;

        let request = MistralChatRequest
        {   model: model.clone()
          , messages: vec![
              ChatMessage
              {   role: "user".to_string()
                , content: prompt
              }
            ]
          , max_tokens: Some(1024)
          , temperature: Some(0.7)
          , stream: Some(false)
        };

        trace!("Mistral request: {:?}", request);

        let response = self.http_client
          .post(format!("{}/chat/completions", MISTRAL_API_BASE))
          .header("Authorization", format!("Bearer {}", api_key))
          .header("Content-Type", "application/json")
          .json(&request)
          .send()
          .await
          .map_err(|e| {
            error!("HTTP error: {}", e);
            crate::error::Error::HttpError(e.to_string())
          })?;

        let status = response.status();
        trace!("Mistral response status: {}", status);

        if !status.is_success()
        {   let error_text = response.text().await
              .unwrap_or_else(|_| 
                "Unknown error".to_string()
              );
            error!("Mistral API error: {}", error_text);
            return Err(crate::error::Error::ApiError(
              format!("Mistral error: {}", error_text)
            ));
        }

        let chat_response: MistralChatResponse
          = response.json().await.map_err(|e| {
            error!("Parse error: {}", e);
            crate::error::Error::ParseError(e.to_string())
          })?;

        chat_response.choices.first()
          .map(|c| c.message.content.clone())
          .ok_or_else(|| {
            error!("No choices in response");
            crate::error::Error::NoChoicesInResponse
          })
    }

    async fn handle_get_models(
      &self
    ) -> Result<Vec<String>, crate::error::Error>
    {   debug!("Handling get_models");

        let api_key = self.master_key.as_ref()
          .ok_or_else(|| {
            error!("No master key");
            crate::error::Error::MissingApiKey(
              "Mistral (master)".to_string()
            )
          })?;

        let response = self.http_client
          .get(format!("{}/models", MISTRAL_API_BASE))
          .header("Authorization", format!("Bearer {}", api_key))
          .send()
          .await
          .map_err(|e| {
            error!("Failed to fetch models: {}", e);
            crate::error::Error::HttpError(e.to_string())
          })?;

        let status = response.status();
        trace!("Models response status: {}", status);

        if !status.is_success()
        {   let error_text = response.text().await
              .unwrap_or_else(|_|
                "Unknown error".to_string()
              );
            error!("Failed to get models: {}", error_text);
            return Err(crate::error::Error::ApiError(
              error_text
            ));
        }

        let models_response: MistralModelsResponse
          = response.json().await.map_err(|e| {
            error!("Parse error: {}", e);
            crate::error::Error::ParseError(e.to_string())
          })?;

        let model_names: Vec<String>
          = models_response.data
            .iter()
            .map(|m| m.id.clone())
            .collect();

        debug!("Retrieved {} models", model_names.len());
        Ok(model_names)
    }

    async fn handle_set_api_key(
      &mut self
    , model_opt: Option<String>
    , key: String
    ) -> Result<(), crate::error::Error>
    {   if let Some(model) = model_opt
        {   self.set_model_key(model, key);
        } else
        {   self.set_master_key(key);
        }
        Ok(())
    }
}

/// Public Mistral client interface
pub struct MistralClient
{   tx: mpsc::UnboundedSender<MistralCommand>
  , _task: tokio::task::JoinHandle<()>
}

impl MistralClient
{   /// Create and spawn a new Mistral client
    pub fn new(
      api_key: Option<String>
    , _error_tx: Option<mpsc::UnboundedSender<
        crate::error::Error
      >>
    ) -> Self
    {   debug!("Creating MistralClient");
        let (cmd_tx, cmd_rx)
          = mpsc::unbounded_channel();

        let _task = tokio::spawn(async move {
          run_mistral_loop(cmd_rx, api_key).await;
        });

        MistralClient
        {   tx: cmd_tx
          , _task
        }
    }

    /// Queue a prompt - returns immediately
    pub async fn send_prompt(
      &self
    , prompt: String
    , model: String
    , reply: mpsc::UnboundedSender<crate::SendPromptReply>
    ) -> Result<(), crate::error::Error>
    {   debug!("send_prompt queued for model: {}", model);
        
        self.tx.send(MistralCommand::SendPrompt {
          prompt,
          model,
          reply,
        }).map_err(|_| {
          error!("Mistral client disconnected");
          crate::error::Error::Other(
            "Mistral client disconnected".to_string()
          )
        })
    }

    /// Queue get_models request
    pub async fn get_available_models(
      &self
    , reply: mpsc::UnboundedSender<
        Result<Vec<String>, crate::error::Error>
      >
    ) -> Result<(), crate::error::Error>
    {   debug!("get_available_models queued");
        
        self.tx.send(MistralCommand::GetModels {
          reply,
        }).map_err(|_| {
          error!("Mistral client disconnected");
          crate::error::Error::Other(
            "Mistral client disconnected".to_string()
          )
        })
    }

    /// Queue set_api_key request
    pub async fn set_api_key(
      &self
    , model: Option<String>
    , key: String
    , reply: mpsc::UnboundedSender<
        Result<(), crate::error::Error>
      >
    ) -> Result<(), crate::error::Error>
    {   debug!("set_api_key queued for model: {:?}", model);
        
        self.tx.send(MistralCommand::SetApiKey {
          model,
          key,
          reply,
        }).map_err(|_| {
          error!("Mistral client disconnected");
          crate::error::Error::Other(
            "Mistral client disconnected".to_string()
          )
        })
    }

    /// Shutdown the client
    pub async fn shutdown(self) 
      -> Result<(), crate::error::Error>
    {   debug!("Shutting down MistralClient");
        self.tx.send(MistralCommand::Shutdown)
          .map_err(|_| {
            crate::error::Error::Other(
              "Client already shutdown".to_string()
            )
          })
    }
}

/// Main mistral event loop
async fn run_mistral_loop(
  mut cmd_rx: mpsc::UnboundedReceiver<MistralCommand>
, api_key: Option<String>
)
{   debug!("Starting Mistral client loop");
    let mut state = MistralClientState::new(api_key);

    loop
    { match cmd_rx.recv().await
      {   Some(MistralCommand::SendPrompt {
            prompt, model, reply
          }) => {
            debug!("Processing SendPrompt");
            let result = state
              .handle_send_prompt(prompt, model)
              .await;
            let _ = reply.send(result);
          }
        , Some(MistralCommand::GetModels { reply }) => {
            debug!("Processing GetModels");
            let result = state.handle_get_models().await;
            let _ = reply.send(result);
          }
        , Some(MistralCommand::SetApiKey {
            model, key, reply
          }) => {
            debug!("Processing SetApiKey for: {:?}", model);
            let result = state
              .handle_set_api_key(model, key)
              .await;
            let _ = reply.send(result);
          }
        , Some(MistralCommand::Shutdown) => {
            info!("Mistral client shutting down");
            break;
          }
        , None => {
            debug!("Command channel closed");
            break;
          }
      }
    }
}

/// Default model info for Mistral
pub fn default_model_info() -> crate::ModelInfo
{   crate::ModelInfo
    {   name: "mistral-small-latest".to_string()
      , max_context_tokens: 32000
      , max_response_tokens: 8000
      , can_save_context: false
      , input_modalities: crate::ModelModalities
        {   supported: vec![
              crate::InputModality::Single(
                crate::BaseModality::Text
              )
            ]
        }
      , supports_streaming: true
      , supports_tools: true
      , provider: crate::Provider::MistralAi
      , default_system_prompt: None
      , supported_file_extensions: None
      , cost_per_million_input_tokens: Some(0.14)
      , cost_per_million_output_tokens: Some(0.42)
      , is_available: true
    }
}