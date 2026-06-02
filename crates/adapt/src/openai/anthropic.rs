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
    anthropic::prefix::{AnthropicChatResponse, AnthropicStreamEvent, AnthropicStreamChunk},
    openai::prefix::{OpenAIChatResponse, OpenAIStreamChunk},
};

use datum::record::TokenUsage;

const DEFAULT_CHUNK_ID: &str = "chatcmpl-anthropic";
const DEFAULT_MODEL_ID: &str = "claude";

// ============ 流式转换: Anthropic → OpenAI ============

pub async fn from_anthropic_streaming(response: Reswponse) -> Result<Response, StatusCode> {
    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let openai_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    None
                } else {
                    // 使用类型化转换器解析Anthropic事件
                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(&event.data) {
                        // 转换为OpenAI流式块并序列化为SSE
                        let chunks = Vec::<OpenAIStreamChunk>::from(&AnthropicStreamChunk {
                            id: DEFAULT_CHUNK_ID.to_string(),
                            model: DEFAULT_MODEL_ID.to_string(),
                            event,
                        });
                        
                        // 转换为字节流
                        Some(Ok::<_, Error>(
                            chunks.into_iter().flat_map(|s| Vec::<u8>::from(&s)).collect::<Vec<u8>>()
                        ))
                    } else {
                        None
                    }
                }
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(openai_stream);
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

    Ok((Json(OpenAIChatResponse::from(&resp)).into_response(), usage))
}