use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

use aidapter::{
    openai::prefix::OpenAIChatResponse,
    anthropic::prefix::AnthropicChatResponse,
    gemini::prefix::GeminiChatResponse,
};


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub requests: u64,
    pub tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageStats {
    pub requests: AtomicU64,
    pub tokens: AtomicU64,
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            requests: AtomicU64::new(0),
            tokens: AtomicU64::new(0),
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
        }
    }
}

impl UsageStats {
    pub fn record(&self, usage: &TokenUsage) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.input_tokens.fetch_add(usage.input_tokens, Ordering::Relaxed);
        self.output_tokens.fetch_add(usage.output_tokens, Ordering::Relaxed);
        self.tokens.fetch_add(usage.tokens, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TokenUsage {
        TokenUsage {
            requests: self.requests.load(Ordering::Relaxed),
            input_tokens: self.input_tokens.load(Ordering::Relaxed),
            output_tokens: self.output_tokens.load(Ordering::Relaxed),
            tokens: self.tokens.load(Ordering::Relaxed),
        }
    }
}

impl From<&OpenAIChatResponse> for TokenUsage {
    fn from(resp: &OpenAIChatResponse) -> Self {
        TokenUsage {
            requests: 1,
            input_tokens: resp.usage.prompt_tokens as u64,
            output_tokens: resp.usage.completion_tokens as u64,
            tokens: resp.usage.total_tokens as u64,
        }
    }
}

impl From<&AnthropicChatResponse> for TokenUsage {
    fn from(resp: &AnthropicChatResponse) -> Self {
        let i = resp.usage.input_tokens as u64;
        let o = resp.usage.output_tokens as u64;
        TokenUsage {
            requests: 1,
            input_tokens: i,
            output_tokens: o,
            tokens: i + o,
        }
    }
}

impl From<&GeminiChatResponse> for TokenUsage {
    fn from(resp: &GeminiChatResponse) -> Self {
        let (p, t) = resp.usage_metadata.as_ref()
            .map(|um| (um.prompt_token_count as u64, um.total_token_count as u64))
            .unwrap_or((0, 0));
        TokenUsage {
            requests: 1,
            input_tokens: p,
            output_tokens: t.saturating_sub(p),
            tokens: t,
        }
    }
}