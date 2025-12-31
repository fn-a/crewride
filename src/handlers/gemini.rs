use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    Json, response::Response, http::StatusCode,
    extract::{State, Path, Query}, response::IntoResponse,
};
use url::Url;
use aidapter::{
    Provider,
    gemini::prefix::GeminiChatRequest,
    openai::prefix::OpenAIChatRequest,
    anthropic::prefix::AnthropicChatRequest,
};

use crate::config::ProxyState;

mod openai;
mod anthropic;

// ============ Gemini 路由处理器 ============

/// 解析 path 参数来提取 model 和 method
/// 格式: {model}:generateContent 或 {model}:streamGenerateContent
pub async fn handler(
    State(state): State<Arc<ProxyState>>,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<GeminiChatRequest>,
) -> Result<Response, StatusCode> {
    // 解析路径：model:method
    let (mut model, method) = match path.rsplit_once(':') {
        Some((m, method)) => (m.to_string(), method),
        None => return Err(StatusCode::BAD_REQUEST),
    };
    
    
    let mut api_key = query.get("key").unwrap_or(&String::new()).to_string();
    let mut provider_config = None;
    let mut replace_config = None;
    // 查找模型配置
    if let Some(model_config) = state.config.find_model(&model) {
        // 如果模型有供应商配置
        if let Some(provider) = &model_config.provider {
            replace_config = model_config.replace.clone();
            provider_config = state.config.find_provider(provider)
        }
    }
    if provider_config.is_none() {
        provider_config = state.config.give_provider(Provider::Gemini);
    }
    let provider_config = provider_config.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut replace_api_key = api_key.is_empty();
    if let Some(replace_config) = replace_config {
        replace_api_key = replace_config.api_key;
        // 使用配置的模型名称替换请求中的模型名称
        if let Some(model_config) = &replace_config.model {
            model = model_config.clone();
        }
    }
    if replace_api_key {
        // 使用配置的API Key替换请求头中的API key
        api_key = provider_config.api_key.clone()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let api_url = provider_config.api_url.clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("📥 Gemini request: model={}, method={}, url={}", model, method, api_url);

    match method {
        "generateContent" => {
            // Gemini API 端点: POST /v1beta/models/{model}:generateContent
            match provider_config.r#type {
                Provider::OpenAI =>  into_openai(state, model, req, api_url, api_key).await,
                Provider::Anthropic => into_anthropic(state, model, req, api_url, api_key).await,
                Provider::Gemini => straight(state, model, req, api_url, api_key).await,
            }
        },
        "streamGenerateContent" => {
            // Gemini API 流式端点: POST /v1beta/models/{model}:streamGenerateContent
            match provider_config.r#type {
                Provider::OpenAI =>  stream::into_openai(state, model, req, api_url, api_key).await,
                Provider::Anthropic => stream::into_anthropic(state, model, req, api_url, api_key).await,
                Provider::Gemini => stream::straight(state, model, req, api_url, api_key).await,
            }
        },
        _ => Err(StatusCode::NOT_FOUND),
    }
}

// ============ Gemini → Gemini 直通 ============

async fn straight(
    state: Arc<ProxyState>,
    model: String,
    req: GeminiChatRequest,
    api_url: Url,
    api_key: String,
) -> Result<Response, StatusCode> {
    println!("⚡ Gemini passthrough");

    let url = api_url
        .join(&format!("/v1beta/models/{}:generateContent?key={}", model, api_key))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = state
        .client
        .post(url.as_str())
        .header("content-type", "application/json")
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    let resp: serde_json::Value = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(resp).into_response())
}

// ============ Gemini → OpenAI ============

async fn into_openai(
    state: Arc<ProxyState>,
    model: String,
    req: GeminiChatRequest,
    api_url: Url,
    api_key: String,
) -> Result<Response, StatusCode> {
    println!("🔄 Gemini → OpenAI");

    // 转换请求: Gemini -> OpenAI
    let mut openai_req = OpenAIChatRequest::from(&req);
    openai_req.model = model;

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
        .json(&openai_req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    openai::from_openai_response(response).await
}

// ============ Gemini → Anthropic ============

async fn into_anthropic(
    state: Arc<ProxyState>,
    model: String,
    req: GeminiChatRequest,
    api_url: Url,
    api_key: String,
) -> Result<Response, StatusCode> {
    println!("🔄 Gemini → Anthropic");

    // 转换请求: Gemini -> Anthropic
    let mut anthropic_req = AnthropicChatRequest::from(&req);
    anthropic_req.model = model;

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
        .json(&anthropic_req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    anthropic::from_anthropic_response(response).await
}

pub mod stream {
    use std::sync::Arc;
    use axum::{
        response::Response, http::StatusCode,
        response::IntoResponse, body::Body,
    };
    use url::Url;
    use aidapter::{
        gemini::prefix::GeminiChatRequest,
        openai::prefix::OpenAIChatRequest,
        anthropic::prefix::AnthropicChatRequest,
    };

    use crate::config::ProxyState;

    use super::{openai, anthropic};

    // ============ Gemini → Gemini 直通Stream ============

    pub async fn straight(
        state: Arc<ProxyState>,
        model: String,
        req: GeminiChatRequest,
        api_url: Url,
        api_key: String,
    ) -> Result<Response, StatusCode> {
        println!("⚡ Gemini streaming passthrough");

        let url = api_url
            .join(&format!("/v1beta/models/{}:streamGenerateContent?key={}&alt=sse", model, api_key))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = state
            .client
            .post(url.as_str())
            .header("content-type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }

        let stream = response.bytes_stream();
        let body = Body::from_stream(stream);
        Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
    }

    // ============ Gemini → OpenAI Stream ============

    pub async fn into_openai(
        state: Arc<ProxyState>,
        model: String,
        req: GeminiChatRequest,
        api_url: Url,
        api_key: String,
    ) -> Result<Response, StatusCode> {
        println!("🔄 Gemini → OpenAI (streaming)");

        // 转换请求: Gemini -> OpenAI，并启用流式
        let mut openai_req = OpenAIChatRequest::from(&req);
        openai_req.model = model;
        openai_req.stream = Some(true);

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
            .json(&openai_req)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }

        openai::from_openai_streaming(response).await
    }

    // ============ Gemini → Anthropic Stream ============

    pub async fn into_anthropic(
        state: Arc<ProxyState>,
        model: String,
        req: GeminiChatRequest,
        api_url: Url,
        api_key: String,
    ) -> Result<Response, StatusCode> {
        println!("🔄 Gemini → Anthropic (streaming)");

        // 转换请求: Gemini -> Anthropic，并启用流式
        let mut anthropic_req = AnthropicChatRequest::from(&req);
        anthropic_req.model = model;
        anthropic_req.stream = Some(true);

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
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }

        anthropic::from_anthropic_streaming(response).await
    }
}