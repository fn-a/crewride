use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use futures::StreamExt;

use aidapter::{
    anthropic::prefix::{AnthropicChatResponse, AnthropicStreamEvent, AnthropicStreamChunk},
    gemini::prefix::{GeminiChatResponse, GeminiStreamChunk},
};

// ============ 流式转换: Anthropic → Gemini ============

pub async fn from_anthropic_streaming(
    response: reqwest::Response,
) -> Result<Response, StatusCode> {
    use eventsource_stream::Eventsource;

    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let gemini_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    return None;
                }

                // 解析Anthropic事件
                let anthropic_event: AnthropicStreamEvent = match serde_json::from_str(&event.data) {
                    Ok(evt) => evt,
                    Err(_) => return None,
                };

                // 构建AnthropicStreamChunk
                let chunk_id = "msg_anthropic";
                let model = "claude";
                let anthropic_chunk = AnthropicStreamChunk {
                    id: chunk_id.to_string(),
                    model: model.to_string(),
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
                
                Some(Ok::<_, std::io::Error>(bytes))
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(gemini_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_anthropic_response(response: reqwest::Response) -> Result<Response, StatusCode> {
    let resp: AnthropicChatResponse = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(GeminiChatResponse::from(&resp)).into_response())
}