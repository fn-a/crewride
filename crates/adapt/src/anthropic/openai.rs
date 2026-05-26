use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use futures::StreamExt;

use aidapter::{
    openai::prefix::{OpenAIChatResponse, OpenAIStreamChunk},
    anthropic::prefix::{AnthropicChatResponse, AnthropicStreamChunk},
};


// ============ 流式转换: OpenAI → Anthropic ============

pub async fn from_openai_streaming(
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

                // 使用类型化转换器解析OpenAI流式响应
                let openai_chunk: OpenAIStreamChunk = match serde_json::from_str(&event.data) {
                    Ok(chunk) => chunk,
                    Err(_) => return None,
                };

                // 转换为Anthropic事件并序列化
                let events = Vec::<AnthropicStreamChunk>::from(&openai_chunk);
                
                // 转换为字节流
                Some(Ok::<_, std::io::Error>(
                    events.into_iter().flat_map(|s| Vec::<u8>::from(&s)).collect::<Vec<u8>>()
                ))
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(anthropic_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_openai_response(response: reqwest::Response) -> Result<Response, StatusCode> {
    let resp: OpenAIChatResponse = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AnthropicChatResponse::from(&resp)).into_response())
}