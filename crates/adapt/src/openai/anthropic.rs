use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use futures::StreamExt;

use aidapter::{
    anthropic::prefix::{AnthropicChatResponse,AnthropicStreamEvent, AnthropicStreamChunk},
    openai::prefix::{OpenAIChatResponse,OpenAIStreamChunk},
};

// ============ 流式转换: Anthropic → OpenAI ============

pub async fn from_anthropic_streaming(
    response: reqwest::Response,
) -> Result<Response, StatusCode> {
    use eventsource_stream::Eventsource;

    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let openai_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    return None;
                }

                // 使用类型化转换器解析Anthropic事件
                let event: AnthropicStreamEvent = match serde_json::from_str(&event.data) {
                    Ok(event) => event,
                    Err(_) => return None,
                };

                // 转换为OpenAI流式块并序列化为SSE
                let chunk_id = "chatcmpl-anthropic";
                let model = "claude";
                let chunks = Vec::<OpenAIStreamChunk>::from(&AnthropicStreamChunk {
                    id: chunk_id.to_string(),
                    model: model.to_string(),
                    event: event,
                });
                
                // 转换为字节流
                Some(Ok::<_, std::io::Error>(
                    chunks.into_iter().flat_map(|s| Vec::<u8>::from(&s)).collect::<Vec<u8>>()
                ))
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(openai_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_anthropic_response(response: reqwest::Response) -> Result<Response, StatusCode> {
    let resp: AnthropicChatResponse  = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(OpenAIChatResponse::from(&resp)).into_response())
}