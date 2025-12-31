use axum::{
    Json, response::Response, http::StatusCode, 
    body::Body, response::IntoResponse
};
use futures::StreamExt;

use aidapter::{
    gemini::prefix::{GeminiChatResponse, GeminiStreamChunk},
    openai::prefix::{OpenAIChatResponse, OpenAIStreamChunk},
};

// ============ 流式转换: Gemini → OpenAI ============

pub async fn from_gemini_streaming(
    response: reqwest::Response,
) -> Result<Response, StatusCode> {
    use eventsource_stream::Eventsource;

    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();

    let openai_stream = event_stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                if event.data == "[DONE]" {
                    return Some(Ok::<_, std::io::Error>(b"data: [DONE]\n\n".to_vec()));
                }

                // 解析Gemini流式响应
                let gemini_chunk: GeminiStreamChunk = match serde_json::from_str(&event.data) {
                    Ok(chunk) => chunk,
                    Err(_) => return None,
                };

                // 转换为OpenAI流式块
                let openai_chunks = Vec::<OpenAIStreamChunk>::from(&gemini_chunk);
                
                // 序列化为OpenAI SSE格式
                let bytes: Vec<u8> = openai_chunks
                    .iter()
                    .flat_map(|chunk| Vec::<u8>::from(chunk))
                    .collect();
                
                Some(Ok::<_, std::io::Error>(bytes))
            }
            Err(_) => None,
        }
    });

    let body = Body::from_stream(openai_stream);
    Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
}

// ============ 非流式响应转换 ============

pub async fn from_gemini_response(response: reqwest::Response) -> Result<Response, StatusCode> {
    let resp: GeminiChatResponse = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(OpenAIChatResponse::from(&resp)).into_response())
}