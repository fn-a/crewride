use std::io::Error;
use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use reqwest::Response as Reswponse;
use futures::StreamExt;
use eventsource_stream::Eventsource;
use anyhow::Result;

use aidapter::{
    openai::prefix::{OpenAIChatResponse, OpenAIStreamChunk},
    anthropic::prefix::{AnthropicChatResponse, AnthropicStreamChunk},
};

use datum::record::TokenUsage;

// ============ 流式转换: OpenAI → Anthropic ============

pub async fn from_openai_streaming(response: Reswponse) -> Result<Response, StatusCode> {
    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let anthropic_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    None
                } else {
                    // 使用类型化转换器解析OpenAI流式响应
                    if let Ok(openai_chunk) = serde_json::from_str::<OpenAIStreamChunk>(&event.data) {
                        // 转换为Anthropic事件并序列化
                        let events = Vec::<AnthropicStreamChunk>::from(&openai_chunk);
                        
                        // 转换为字节流
                        Some(Ok::<_, Error>(
                            events.into_iter().flat_map(|s| Vec::<u8>::from(&s)).collect::<Vec<u8>>()
                        ))
                    } else {
                        None
                    }
                }
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(anthropic_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_openai_response(response: Reswponse) -> Result<(Response, TokenUsage), StatusCode> {
    let resp: OpenAIChatResponse = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let usage = TokenUsage {
        requests: 1,
        input_tokens: resp.usage.prompt_tokens as u64,
        output_tokens: resp.usage.completion_tokens as u64,
        tokens: resp.usage.total_tokens as u64,
    };
    
    Ok((Json(AnthropicChatResponse::from(&resp)).into_response(), usage))
}