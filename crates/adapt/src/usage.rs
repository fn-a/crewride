use std::ops::Deref;

use aidapter::{
    openai::prefix::OpenAIChatResponse,
    anthropic::prefix::AnthropicChatResponse,
    gemini::prefix::GeminiChatResponse,
};

use datum::TokenUsage;

pub struct TokenExtract(TokenUsage);

impl From<&OpenAIChatResponse> for TokenExtract {
    fn from(resp: &OpenAIChatResponse) -> Self {
        Self(TokenUsage {
            requests: 1,
            input_tokens: resp.usage.prompt_tokens as u64,
            output_tokens: resp.usage.completion_tokens as u64,
            tokens: resp.usage.total_tokens as u64,
        })
    }
}

impl From<&AnthropicChatResponse> for TokenExtract {
    fn from(resp: &AnthropicChatResponse) -> Self {
        let i = resp.usage.input_tokens as u64;
        let o = resp.usage.output_tokens as u64;
        Self(TokenUsage {
            requests: 1,
            input_tokens: i,
            output_tokens: o,
            tokens: i + o,
        })
    }
}

impl From<&GeminiChatResponse> for TokenExtract {
    fn from(resp: &GeminiChatResponse) -> Self {
        let (p, t) = resp.usage_metadata.as_ref()
            .map(|um| (um.prompt_token_count as u64, um.total_token_count as u64))
            .unwrap_or((0, 0));
        Self(TokenUsage {
            requests: 1,
            input_tokens: p,
            output_tokens: t.saturating_sub(p),
            tokens: t,
        })
    }
}

impl AsRef<TokenUsage> for TokenExtract {
    fn as_ref(&self) -> &TokenUsage {
        &self.0
    }
}

impl Deref for TokenExtract {
    type Target = TokenUsage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn extract<T>(resp: T) -> TokenUsage where TokenExtract: From<T> 
{
    let ext = TokenExtract::from(resp);
    ext.0
}