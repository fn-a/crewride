//! CrewRide - 双向流式代理服务器
//!
//! 支持OpenAI，Anthropic，Gemini API之间的无缝切换和转换

use reqwest::Client;
use datum::{record::UsageStats, config::Config};

pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod retry;

pub struct AdaptState {
    pub client: Client,
    pub config: Config,
    pub stats: UsageStats,
}