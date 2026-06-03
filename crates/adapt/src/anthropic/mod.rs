use std::sync::Arc;
use axum::{
    Json, response::Response, http::StatusCode,
    extract::State, response::IntoResponse,
    http::HeaderMap,
};
use url::Url;
use anyhow::Result;
use aidapter::{
    Provider,
    anthropic::prefix::{
        AnthropicChatRequest, AnthropicChatResponse, 
        AnthropicModelList, AnthropicModelInfo,
    },
    openai::prefix::OpenAIChatRequest,
    gemini::prefix::GeminiChatRequest,
};

use datum::{config::RetryConfig, record::TokenUsage};

use crate::{AdaptState, retry};

pub mod gemini;
pub mod openai;

// ============ Anthropic 端点处理器 ============

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AdaptState>>,
    Json(mut req): Json<AnthropicChatRequest>,
) -> Result<Response, StatusCode> {
    // 提取请求头中的API key
    let mut api_key = headers.get("x-api-key")
        .and_then(|key| key.to_str().ok())
        .map(|key| key.to_string())
        .unwrap_or_default();
    let mut provider_config = None;
    let mut replace_api_key = api_key.is_empty();
    // 查找模型配置
    if let Some(model_config) = state.config.find_model(&req.model) {
        // BYOK 表示使用用户携带的 API Key，将不进行替换
        replace_api_key = !model_config.byokey;
        // 使用配置的模型名称替换请求中的模型名称
        if let Some(remodel) = &model_config.remodel {
            req.model = remodel.clone();
        }
        // 如果模型有供应商配置
        if let Some(provider) = &model_config.provider {
            provider_config = state.config.find_provider(provider)
        }
    }
    if provider_config.is_none() {
        provider_config = state.config.give_provider(Provider::Anthropic);
    }
    let provider_config = provider_config.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if replace_api_key {
        // 使用配置的 API Key 替换请求头中的 API key
        api_key = provider_config.api_key.clone()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let api_url = provider_config.api_url.clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("📥 Anthropic request: model={}, url={}", req.model, api_url);

    let retry = provider_config.retry.clone();
    match provider_config.r#type {
        Provider::Anthropic => straight(state, req, api_url, api_key, retry.as_ref()).await,
        Provider::OpenAI => into_openai(state, req, api_url, api_key, retry.as_ref()).await,
        Provider::Gemini => into_gemini(state, req, api_url, api_key, retry.as_ref()).await,
    }
}

pub fn models(state: &AdaptState) -> AnthropicModelList {
    let data = state.config.models
        .iter()
        .map(|m| AnthropicModelInfo {
            id: m.model.clone(),
            model_type: "model".into(),
            display_name: m.name.clone().unwrap_or_else(|| m.model.clone()),
            created_at: "2024-01-01T00:00:00Z".into(),
            capabilities: None,
        })
        .collect::<Vec<_>>();
    let first_id = data.first().map(|e| e.id.clone());
    let last_id = data.last().map(|e| e.id.clone());
    AnthropicModelList { data, has_more: false, first_id, last_id }
}

// ============ Anthropic → Anthropic 直通 ============

async fn straight(
    state: Arc<AdaptState>,
    req: AnthropicChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    let streaming = req.stream.unwrap_or(false);
    println!("⚡ Anthropic passthrough (stream={})", streaming);

    let api_url = api_url
        .join("/v1/messages")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, ||
        state.client
            .post(api_url.clone())
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&req),
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        if streaming {
            let stream = response.bytes_stream();
            let body = axum::body::Body::from_stream(stream);
            Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
        } else {
            let resp = response
                .json::<AnthropicChatResponse>()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            state.stats.record(&TokenUsage::from(&resp));
            Ok(Json(resp).into_response())
        }
    }

}

// ============ Anthropic → OpenAI ============

async fn into_openai(
    state: Arc<AdaptState>,
    req: AnthropicChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    let streaming = req.stream.unwrap_or(false);
    println!("🔄 Anthropic → OpenAI (stream={})", streaming);

    let api_url = api_url
        .join("/v1/chat/completions")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, ||
        state.client
            .post(api_url.clone())
            .header("authorization", format!("Bearer {}", &api_key))
            .header("content-type", "application/json")
            .json(&OpenAIChatRequest::from(&req))
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        if streaming {
            openai::from_openai_streaming(response).await
        } else {
            let (resp, usage) = openai::from_openai_response(response).await?;
            state.stats.record(&usage);
            Ok(resp)
        }
    }

}

// ============ Anthropic → Gemini ============

async fn into_gemini(
    state: Arc<AdaptState>,
    req: AnthropicChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    let streaming = req.stream.unwrap_or(false);
    println!("🔄 Anthropic → Gemini (stream={})", streaming);

    let gemini_req = GeminiChatRequest::from(&req);

    let endpoint = if streaming {
        format!("/v1beta/models/{}:streamGenerateContent?key={}&alt=sse", req.model, api_key)
    } else {
        format!("/v1beta/models/{}:generateContent?key={}", req.model, api_key)
    };

    let api_url = api_url
        .join(&endpoint)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, ||
        state.client
            .post(api_url.clone())
            .header("content-type", "application/json")
            .json(&gemini_req)
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        if streaming {
            gemini::from_gemini_streaming(response).await
        } else {
            let (resp, usage) = gemini::from_gemini_response(response).await?;
            state.stats.record(&usage);
            Ok(resp)
        }
    }

}