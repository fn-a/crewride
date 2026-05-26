use std::io::Error;
use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use reqwest::Response as Reswponse;
use futures::StreamExt;
use eventsource_stream::Eventsource;

use aidapter::{
    anthropic::prefix::{AnthropicChatResponse, AnthropicStreamEvent, AnthropicStreamChunk},
    gemini::prefix::{GeminiChatResponse, GeminiStreamChunk},
};

use datum::TokenUsage;

const DEFAULT_CHUNK_ID: &str = "msg_anthropic";
const DEFAULT_MODEL_ID: &str = "claude";

// ============ 流式转换: Anthropic → Gemini ============

pub async fn from_anthropic_streaming(response: Reswponse) -> Result<Response, StatusCode> {
    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let gemini_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    None
                } else {
                    // 解析Anthropic事件
                    if let Ok(anthropic_event) = serde_json::from_str::<AnthropicStreamEvent>(&event.data) {
                        // 构建AnthropicStreamChunk
                        let anthropic_chunk = AnthropicStreamChunk {
                            id: DEFAULT_CHUNK_ID.to_string(),
                            model: DEFAULT_MODEL_ID.to_string(),
                            event: anthropic_event,
                        };
        
                        // 转换为Gemini流式块
                        let gemini_chunks = Vec::<GeminiStreamChunk>::from(&anthropic_chunk);
                        
                        // 序列化为Gemini SSE格式
                        let bytes: Vec<u8> = gemini_chunks
                            .iter()
                            .flat_map(|chunk| {
                                let json = serde_json::to_string(chunk).unwrap_or_default();
                                format!("data: {}\n\n", json).into_bytes()
                            })
                            .collect();
                        
                        Some(Ok::<_, Error>(bytes))
                    } else {
                        None
                    }
                }
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(gemini_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_anthropic_response(response: Reswponse) -> Result<(Response, TokenUsage), StatusCode> {
    let resp: AnthropicChatResponse = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let usage = TokenUsage {
        requests: 1,
        input_tokens: resp.usage.input_tokens as u64,
        output_tokens: resp.usage.output_tokens as u64,
        tokens: (resp.usage.input_tokens + resp.usage.output_tokens) as u64,
    };

    Ok((Json(GeminiChatResponse::from(&resp)).into_response(), usage))
}