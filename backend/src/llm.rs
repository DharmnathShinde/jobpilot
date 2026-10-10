//! Minimal Anthropic Messages API client. Optional: every caller has a rule-based fallback.

use anyhow::{anyhow, Context};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct Llm {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl Llm {
    pub fn new(http: reqwest::Client, api_key: String, model: String) -> Self {
        Self { http, api_key, model }
    }

    pub async fn complete(&self, system: &str, user: &str, max_tokens: u32) -> anyhow::Result<String> {
        let res = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": self.model,
                "max_tokens": max_tokens,
                "system": system,
                "messages": [{ "role": "user", "content": user }],
            }))
            .send()
            .await
            .context("LLM request failed")?;
        let status = res.status();
        let body: Value = res.json().await.context("LLM response was not JSON")?;
        if !status.is_success() {
            return Err(anyhow!("LLM returned {status}: {body}"));
        }
        let text = body["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find(|b| b["type"] == "text"))
            .and_then(|b| b["text"].as_str())
            .ok_or_else(|| anyhow!("LLM response had no text"))?;
        Ok(text.to_string())
    }

    /// Asks for JSON and parses the first {...} object in the reply.
    pub async fn complete_json(&self, system: &str, user: &str, max_tokens: u32) -> anyhow::Result<Value> {
        let text = self.complete(system, user, max_tokens).await?;
        let start = text.find('{').ok_or_else(|| anyhow!("no JSON object in LLM reply"))?;
        let end = text.rfind('}').ok_or_else(|| anyhow!("no JSON object in LLM reply"))?;
        Ok(serde_json::from_str(&text[start..=end])?)
    }
}
