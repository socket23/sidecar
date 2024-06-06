use std::collections::HashMap;

pub struct PareaClient {
    client: reqwest::Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PareaLogMessage {
    role: String,
    content: String,
}

impl PareaLogMessage {
    pub fn new(role: String, content: String) -> Self {
        Self { role, content }
    }
}

#[derive(Debug, Clone)]
pub struct PareaLogCompletion {
    messages: Vec<PareaLogMessage>,
    metadata: HashMap<String, String>,
    response: String,
    temperature: f32,
    parent_trace_id: String,
    trace_id: String,
    root_trace_id: String,
    llm: String,
    provider: String,
    trace_name: String,
}

impl PareaLogCompletion {
    pub fn new(
        messages: Vec<PareaLogMessage>,
        metadata: HashMap<String, String>,
        response: String,
        temperature: f32,
        trace_id: String,
        parent_trace_id: String,
        root_trace_id: String,
        llm: String,
        provider: String,
        trace_name: String,
    ) -> Self {
        Self {
            messages,
            metadata,
            response,
            temperature,
            parent_trace_id,
            trace_id,
            root_trace_id,
            llm,
            provider,
            trace_name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PareaLogEvent {
    metadata: HashMap<String, String>,
    event_name: String,
    parent_trace_id: String,
    trace_id: String,
    root_trace_id: String,
}

impl PareaLogEvent {
    pub fn new(
        event_name: String,
        metadata: HashMap<String, String>,
        trace_id: String,
        parent_trace_id: String,
        root_trace_id: String,
    ) -> Self {
        Self {
            event_name,
            metadata,
            parent_trace_id,
            trace_id,
            root_trace_id,
        }
    }
}

impl PareaClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn log_completion(&self, completion: PareaLogCompletion) {
        let url =
            "https://parea-ai-backend-us-9ac16cdbc7a7b006.onporter.run/api/parea/v1/trace_log";

        let mut metadata = completion.metadata.clone();
        metadata.insert(
            "evaluation_metric_names".to_owned(),
            "XML Checker".to_owned(),
        );

        let body = serde_json::json!({
            "trace_id": completion.trace_id,
            "root_trace_id": completion.root_trace_id,
            "parent_trace_id": completion.parent_trace_id,
            "trace_name": completion.trace_name,
            "project_name": "default",
            "inputs": completion.metadata,
            "output": completion.response,
            "configuration": {
                "model": completion.llm,
                "provider": completion.provider,
                "messages": completion.messages,
                "temperature": completion.temperature,
            },
            "start_timestamp": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            "end_timestamp": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            "status": "success",
            "metadata": metadata,
            "depth": 0,
            "execution_order": 0,
            "evaluation_metric_names": vec!["XML Checker"],
        });

        let _ = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header(
                "x-api-key",
                "pai-fadddd1b1f5ad7b39b082541ef715fb9b0017a77125b0225c3e778acfc43c206",
            )
            .body(serde_json::to_string(&body).expect("conversion should never fail for logging"))
            .send()
            .await;
    }
}
