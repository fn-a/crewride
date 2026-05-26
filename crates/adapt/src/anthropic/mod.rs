use std::sync::Arc;
use axum::{
    Json, response::Response, http::StatusCode,
    extract::State, response::IntoResponse,
    http::HeaderMap,
};
use url::Url;
use aidapter::{
    Provider,
    anthropic::prefix::AnthropicChatRequest,
    openai::prefix::OpenAIChatRequest,
    gemini::prefix::GeminiChatRequest,
};

use datum::ProxyState;

pub mod gemini;
pub mod openai;

// ============ Anthropic 端点处理器 ============

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<ProxyState>>,
    Json(mut req): Json<AnthropicChatRequest>,
) -> Result<Response, StatusCode> {
    // 提取请求头中的API key
    let mut api_key = headers.get("x-api-key")
        .and_then(|key| key.to_str().ok())
        .map(|key| key.to_string())
        .unwrap_or_default();
    let mut provider_config = None;
    let mut replace_config = None;
    // 查找模型配置
    if let Some(model_config) = state.config.find_model(&req.model) {
        // 如果模型有供应商配置
        if let Some(provider) = &model_config.provider {
            replace_config = model_config.replace.clone();
            provider_config = state.config.find_provider(provider)
        }
    }
    if provider_config.is_none() {
        provider_config = state.config.give_provider(Provider::Anthropic);
    }
    let provider_config = provider_config.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut replace_api_key = api_key.is_empty();
    if let Some(replace_config) = replace_config {
        replace_api_key = replace_config.api_key;
        // 使用配置的模型名称替换请求中的模型名称
        if let Some(model_config) = &replace_config.model {
            req.model = model_config.clone();
        }
    }
    if replace_api_key {
        // 使用配置的API Key替换请求头中的API key
        api_key = provider_config.api_key.clone()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let api_url = provider_config.api_url.clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("📥 Anthropic request: model={}, url={}", req.model, api_url);

    match provider_config.r#type {
        Provider::Anthropic => straight(state, req, api_url, api_key).await,
        Provider::OpenAI => into_openai(state, req, api_url, api_key).await,
        Provider::Gemini => into_gemini(state, req, api_url, api_key).await,
    }
}

// ============ Anthropic → Anthropic 直通 ============

async fn straight(
    state: Arc<ProxyState>,
    req: AnthropicChatRequest,
    api_url: Url,
    api_key: String,
) -> Result<Response, StatusCode> {
    let is_streaming = req.stream.unwrap_or(false);
    println!("⚡ Anthropic passthrough (stream={})", is_streaming);

    let response = state
        .client
        .post(
            api_url
                .join("/v1/messages")
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .as_str(),
        )
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    if is_streaming {
        let stream = response.bytes_stream();
        let body = axum::body::Body::from_stream(stream);
        Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
    } else {
        let resp: serde_json::Value = response
            .json()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(resp).into_response())
    }
}

// ============ Anthropic → OpenAI ============

async fn into_openai(
    state: Arc<ProxyState>,
    req: AnthropicChatRequest,
    api_url: Url,
    api_key: String,
) -> Result<Response, StatusCode> {
    let is_streaming = req.stream.unwrap_or(false);
    println!("🔄 Anthropic → OpenAI (stream={})", is_streaming);

    let response = state
        .client
        .post(
            api_url
                .join("/v1/chat/completions")
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .as_str(),
        )
        .header("authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&OpenAIChatRequest::from(&req))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    if is_streaming {
        openai::from_openai_streaming(response).await
    } else {
        openai::from_openai_response(response).await
    }
}

// ============ Anthropic → Gemini ============

async fn into_gemini(
    state: Arc<ProxyState>,
    req: AnthropicChatRequest,
    api_url: Url,
    api_key: String,
) -> Result<Response, StatusCode> {
    let is_streaming = req.stream.unwrap_or(false);
    println!("🔄 Anthropic → Gemini (stream={})", is_streaming);

    // 转换请求: Anthropic -> Gemini
    let gemini_req = GeminiChatRequest::from(&req);

    // 构建URL，根据是否流式选择不同端点
    let endpoint = if is_streaming {
        format!("/v1beta/models/{}:streamGenerateContent?key={}&alt=sse", req.model, api_key)
    } else {
        format!("/v1beta/models/{}:generateContent?key={}", req.model, api_key)
    };

    let response = state
        .client
        .post(
            api_url
                .join(&endpoint)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .as_str(),
        )
        .header("content-type", "application/json")
        .json(&gemini_req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    if is_streaming {
        gemini::from_gemini_streaming(response).await
    } else {
        gemini::from_gemini_response(response).await
    }
}