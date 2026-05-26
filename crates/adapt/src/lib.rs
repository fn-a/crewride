//! CrewRide - 双向流式代理服务器
//!
//! 支持OpenAI，Anthropic，Gemini API之间的无缝切换和转换

pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod retry;
pub mod usage;