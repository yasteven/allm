use std::collections::HashMap;
use tokio::sync::mpsc;
use log::{debug, trace, error, info};
use crate::AllmFoot;

/// Union of all possible handler commands to execute
pub enum HandlerCommand
{   SendPrompt
    {   prompt: String
      , model: String
      , reply_id: usize
    }
  , SetApiKeys(Vec<crate::ApiKeySpec>)
  , GetModelLists
  , SetModelFallbackPreference(Vec<(crate::Provider, String)>)
}

/// Backend state machine for managing LLM requests
pub struct AllmBackendState
{   pub current_model: (crate::Provider, crate::ModelInfo)
  , pub api_keys: HashMap<(crate::Provider, String), String>
  , pub fallback_preferences
      : Vec<(crate::Provider, String)>
  , pub mistral_client: crate::providers::mistral::MistralClient
}

impl AllmBackendState
{   /// Create a new backend state with default configuration
    pub fn new(
      mistral_api_key: Option<String>
    ) -> Self
    {   debug!("Initializing AllmBackendState");
        let mistral_client
          = crate::providers::mistral::MistralClient::new(
              mistral_api_key,
              None
            );
        AllmBackendState
        {   current_model: (
              crate::Provider::MistralAi
            , crate::providers::mistral::default_model_info()
            )
          , api_keys: HashMap::new()
          , fallback_preferences: vec![]
          , mistral_client
        }
    }
}

/// Public API for ALLM backend - owns the task
pub struct AllmBackend
{   hand: crate::AllmHand
  , _task_handle: tokio::task::JoinHandle<()>
}

impl AllmBackend
{   /// Create and spawn a new ALLM backend
    /// Returns immediately - spawns background task
    pub fn new(
      mistral_api_key: Option<String>
    ) -> Self
    {   debug!("Creating AllmBackend with task ownership");
        
        let (send_prompt_tx, send_prompt_rx)
          = mpsc::unbounded_channel();
        let (set_api_keys_tx, set_api_keys_rx)
          = mpsc::unbounded_channel();
        let (get_model_lists_tx, get_model_lists_rx)
          = mpsc::unbounded_channel();
        let (kill_process_tx, kill_process_rx)
          = mpsc::unbounded_channel();
        let (set_model_fallback_preference_tx
             , set_model_fallback_preference_rx)
          = mpsc::unbounded_channel();

        let hand = crate::AllmHand
        {   send_prompt_tx: send_prompt_tx.clone()
          , set_api_keys_tx: set_api_keys_tx.clone()
          , get_model_lists_tx: get_model_lists_tx.clone()
          , kill_process_tx: kill_process_tx.clone()
          , set_model_fallback_preference_tx
              : set_model_fallback_preference_tx.clone()
        };

        let foot = crate::AllmFoot
        {   send_prompt_rx
          , set_api_keys_rx
          , get_model_lists_rx
          , kill_process_rx
          , set_model_fallback_preference_rx
        };

        let _task_handle = tokio::spawn(async move {
          run_backend_loop(foot, mistral_api_key).await
        });

        AllmBackend
        {   hand
          , _task_handle
        }
    }

    /// Send a prompt - returns almost immediately
    pub async fn send_prompt(
      &self
    , prompt: String
    , model: String
    ) -> Result<
        mpsc::UnboundedReceiver<crate::SendPromptReply>,
        crate::error::Error
      >
    {   debug!("send_prompt queuing command for model: {}", model);
        let (reply_tx, reply_rx)
          = mpsc::unbounded_channel();
        
        let cmd = crate::SendPromptArgs
        {   prompt
          , model
          , reply: reply_tx
        };

        self.hand.send_prompt_tx
          .send(cmd)
          .map_err(|_| {
            error!("Backend channel closed");
            crate::error::Error::Other(
              "Backend disconnected".to_string()
            )
          })?;

        Ok(reply_rx)
    }

    /// Set API keys - returns almost immediately
    pub async fn set_api_keys(
      &self
    , keys: Vec<crate::ApiKeySpec>
    ) -> Result<
        mpsc::UnboundedReceiver<crate::SetApiKeysReply>,
        crate::error::Error
      >
    {   debug!("set_api_keys queuing {} keys", keys.len());
        let (reply_tx, reply_rx)
          = mpsc::unbounded_channel();
        
        let cmd = crate::SetApiKeysArgs
        {   keys
          , reply: reply_tx
        };

        self.hand.set_api_keys_tx
          .send(cmd)
          .map_err(|_| {
            error!("Backend channel closed");
            crate::error::Error::Other(
              "Backend disconnected".to_string()
            )
          })?;

        Ok(reply_rx)
    }

    /// Get model lists - returns almost immediately
    pub async fn get_model_lists(
      &self
    ) -> Result<
        mpsc::UnboundedReceiver<crate::GetModelListsReply>,
        crate::error::Error
      >
    {   debug!("get_model_lists queuing command");
        let (reply_tx, reply_rx)
          = mpsc::unbounded_channel();
        
        let cmd = crate::GetModelListsArgs
        {   reply: reply_tx
        };

        self.hand.get_model_lists_tx
          .send(cmd)
          .map_err(|_| {
            error!("Backend channel closed");
            crate::error::Error::Other(
              "Backend disconnected".to_string()
            )
          })?;

        Ok(reply_rx)
    }

    /// Set fallback preference - returns almost immediately
    pub async fn set_model_fallback_preference(
      &self
    , preferences: Vec<(crate::Provider, String)>
    ) -> Result<
        mpsc::UnboundedReceiver
          <crate::SetModelFallbackPreferenceReply>,
        crate::error::Error
      >
    {   debug!("set_model_fallback_preference queuing");
        let (reply_tx, reply_rx)
          = mpsc::unbounded_channel();
        
        let cmd = crate::SetModelFallbackPreferenceArgs
        {   preferences
          , reply: reply_tx
        };

        self.hand.set_model_fallback_preference_tx
          .send(cmd)
          .map_err(|_| {
            error!("Backend channel closed");
            crate::error::Error::Other(
              "Backend disconnected".to_string()
            )
          })?;

        Ok(reply_rx)
    }

    /// Gracefully shutdown the backend
    pub async fn shutdown(self) 
      -> Result<(), crate::error::Error>
    {   debug!("Shutting down AllmBackend");
        let (reply_tx, mut reply_rx)
          = mpsc::unbounded_channel();
        
        let cmd = crate::KillProcessArgs
        {   reply: reply_tx
        };

        self.hand.kill_process_tx
          .send(cmd)
          .map_err(|_| {
            error!("Backend channel already closed");
            crate::error::Error::Other(
              "Backend already shutdown".to_string()
            )
          })?;

        // Wait for shutdown confirmation
        if let Some(result) = reply_rx.recv().await
        {   debug!("Backend shutdown confirmed");
            result
        } else
        {   error!("Backend shutdown timeout");
            Err(crate::error::Error::Timeout)
        }
    }
}

/// Main backend event loop
/// 
/// Design: tokio::select! is ONLY for fast queueing.
/// Each select arm immediately routes to the right handler
/// (in this case: mistral) and returns. No awaiting on work.
async fn run_backend_loop(
  foot: crate::AllmFoot
, mistral_api_key: Option<String>
)
{   debug!("Starting AllmBackend event loop");
    let mut state = AllmBackendState::new(mistral_api_key);
    let AllmFoot
    {   mut send_prompt_rx
      , mut set_api_keys_rx
      , mut get_model_lists_rx
      , mut kill_process_rx
      , mut set_model_fallback_preference_rx
    } = foot;

    loop
    { tokio::select!
      { Some(cmd) = send_prompt_rx.recv() => {
          debug!("Received SendPrompt for model: {}", cmd.model);
          
          // Route to appropriate provider
          match state.current_model.0
          {   crate::Provider::MistralAi => {
                let _ = state.mistral_client
                  .send_prompt(
                    cmd.prompt,
                    cmd.model,
                    cmd.reply
                  )
                  .await;
              }
            , _ => {
                error!("Provider not implemented");
                let _ = cmd.reply.send(
                  Err(crate::error::Error::ProviderNotImplemented(
                    format!("{:?}", state.current_model.0)
                  ))
                );
              }
          }
        }
      , Some(cmd) = set_api_keys_rx.recv() => {
          debug!("Received SetApiKeys");
          for key_spec in cmd.keys
          {   state.api_keys.insert(
                (key_spec.provider, key_spec.model),
                key_spec.key
              );
          }
          let _ = cmd.reply.send(Ok(()));
        }
      , Some(cmd) = get_model_lists_rx.recv() => {
          debug!("Received GetModelLists");
          let _ = cmd.reply.send(Ok(vec![]));
        }
      , Some(cmd) = kill_process_rx.recv() => {
          debug!("Received KillProcess");
          let _ = cmd.reply.send(Ok(()));
          info!("AllmBackend shutting down");
          break;
        }
      , Some(cmd) = set_model_fallback_preference_rx.recv() => {
          debug!("Received SetModelFallbackPreference");
          state.fallback_preferences = cmd.preferences;
          let _ = cmd.reply.send(Ok(()));
        }
      }
    }
}