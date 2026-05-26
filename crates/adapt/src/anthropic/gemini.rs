use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use futures::StreamExt;

use aidapter::{
    gemini::prefix::{GeminiChatResponse, GeminiStreamChunk},
    anthropic::prefix::{AnthropicChatResponse, AnthropicStreamChunk},
};

// ============ 流式转换: Gemini → Anthropic ============

pub async fn from_gemini_streaming(
    response: reqwest::Response,
) -> Result<Response, StatusCode> {
    use eventsource_stream::Eventsource;

    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let anthropic_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    return None;
                }

                // 解析Gemini流式响应
                let gemini_chunk: GeminiStreamChunk = match serde_json::from_str(&event.data) {
                    Ok(chunk) => chunk,
                    Err(_) => return None,
                };

                // 转换为Anthropic流式块
                let anthropic_chunks = Vec::<AnthropicStreamChunk>::from(&gemini_chunk);
                
                // 序列化为Anthropic SSE格式
                let bytes: Vec<u8> = anthropic_chunks
                    .iter()
                    .flat_map(|chunk| Vec::<u8>::from(chunk))
                    .collect();
                
                Some(Ok::<_, std::io::Error>(bytes))
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(anthropic_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_gemini_response(response: reqwest::Response) -> Result<Response, StatusCode> {
    let resp: GeminiChatResponse = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AnthropicChatResponse::from(&resp)).into_response())
}