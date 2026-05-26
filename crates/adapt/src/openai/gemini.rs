use std::io::Error;
use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use reqwest::Response as Reswponse;
use futures::StreamExt;
use eventsource_stream::Eventsource;

use aidapter::{
    gemini::prefix::{GeminiChatResponse, GeminiStreamChunk},
    openai::prefix::{OpenAIChatResponse, OpenAIStreamChunk},
};

use datum::TokenUsage;

// ============ 流式转换: Gemini → OpenAI ============

pub async fn from_gemini_streaming(response: Reswponse) -> Result<Response, StatusCode> {
    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let openai_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    Some(Ok::<_, std::io::Error>(b"data: [DONE]\n\n".to_vec()))
                } else {
                    // 解析Gemini流式响应
                    if let Ok(gemini_chunk) = serde_json::from_str::<GeminiStreamChunk>(&event.data) {
                        // 转换为OpenAI流式块
                        let openai_chunks = Vec::<OpenAIStreamChunk>::from(&gemini_chunk);
                        
                        // 序列化为OpenAI SSE格式
                        let bytes: Vec<u8> = openai_chunks
                            .iter()
                            .flat_map(|chunk| Vec::<u8>::from(chunk))
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

    let body = Body::from_stream(openai_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_gemini_response(response: Reswponse) -> Result<(Response, TokenUsage), StatusCode> {
    let resp: GeminiChatResponse = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let usage = resp.usage_metadata.as_ref().map(|um| {
        let prompt = um.prompt_token_count as u64;
        let total = um.total_token_count as u64;
        TokenUsage {
            requests: 1,
            input_tokens: prompt,
            output_tokens: total.saturating_sub(prompt),
            tokens: total,
        }
    }).unwrap_or_default();
    Ok((Json(OpenAIChatResponse::from(&resp)).into_response(), usage))
}