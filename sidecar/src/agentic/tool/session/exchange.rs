//! Exposes a client to start a new exchange when we want it, can be used by the
//! agent to send replies, followings etc

use async_trait::async_trait;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionExchangeNewRequest {
    session_id: String,
    editor_url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SessionExchangeNewResponse {
    exchange_id: String,
}

pub struct SessionExchangeClient {
    client: reqwest::Client,
}

impl SessionExchangeClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for SessionExchangeClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_new_exchange_during_session()?;
        let endpoint = context.editor_url.to_owned() + "/new_exchange";
        let response = self
            .client
            .post(endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let new_exchange: SessionExchangeNewResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::new_exchange_during_session(new_exchange))
    }
}
