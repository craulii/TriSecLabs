use common::AppError;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

#[derive(Clone)]
pub struct LlmClient {
    inner: reqwest::Client,
    base_url: String,
    model: String,
}

// Tipos compatibles con la API OpenAI que expone llama.cpp server

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

impl LlmClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let inner = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build reqwest client");
        Self {
            inner,
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    /// Envía un prompt al LLM y retorna la respuesta completa.
    /// El caller es responsable de estructurar el prompt correctamente.
    pub async fn complete(&self, system: &str, user_prompt: &str) -> Result<String, AppError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage { role: "system", content: system },
                ChatMessage { role: "user", content: user_prompt },
            ],
            max_tokens: 1024,
            temperature: 0.3,
            stream: false,
        };

        debug!(model = %self.model, "sending request to LLM");

        let response = self
            .inner
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "LLM request failed");
                AppError::Llm(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!("HTTP {status}: {body}")));
        }

        let parsed: ChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("failed to parse response: {e}")))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| AppError::Llm("empty response from LLM".into()))
    }
}
