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
    gemini::prefix::{GeminiChatResponse, GeminiStreamChunk},
};

use datum::record::TokenUsage;

// ============ 流式转换: OpenAI → Gemini ============

pub async fn from_openai_streaming(response: Reswponse) -> Result<Response, StatusCode> {
    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let gemini_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    return None;
                } else {
                    // 解析OpenAI流式响应
                    if let Ok(openai_chunk) = serde_json::from_str::<OpenAIStreamChunk>(&event.data) {
                        // 转换为Gemini流式块
                        let gemini_chunks = Vec::<GeminiStreamChunk>::from(&openai_chunk);
                        
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

    Ok((Json(GeminiChatResponse::from(&resp)).into_response(), usage))
}