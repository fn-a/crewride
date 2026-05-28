use std::sync::Arc;
use axum::{
    Json, response::Response, http::StatusCode,
    extract::State, body::Body, response::IntoResponse,
    http::{ HeaderMap, header::AUTHORIZATION }
};
use url::Url;
use aidapter::{
    Provider,
    openai::prefix::{
        OpenAIChatRequest, OpenAIChatResponse,
        OpenAIModelList, OpenAIModelInfo,
    },
    anthropic::prefix::AnthropicChatRequest,
    gemini::prefix::GeminiChatRequest,
};

use datum::{AdaptState, RetryConfig};

use crate::{retry, usage};

pub mod anthropic;
pub mod gemini;

// ============ OpenAI 端点处理器 ============

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AdaptState>>,
    Json(mut req): Json<OpenAIChatRequest>,
) -> Result<Response, StatusCode> {
    // 提取请求头中的API key
    let mut api_key = headers.get(AUTHORIZATION)
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .map(|key| key.to_string())
        .unwrap_or_default();
    let mut provider_config = None;
    let mut replace_api_key = api_key.is_empty();
    // 查找模型配置
    if let Some(model_config) = state.config.find_model(&req.model) {
        // BYOK 表示使用用户携带的 API Key，将不进行替换
        replace_api_key = !model_config.byokey;
        // 如果配置了替换模型
        if let Some(model) = &model_config.remodel {
            req.model = model.clone();
        }
        // 如果模型有供应商配置
        if let Some(provider) = &model_config.provider {
            provider_config = state.config.find_provider(provider)
        }
    }
    if provider_config.is_none() {
        provider_config = state.config.give_provider(Provider::OpenAI);
    }
    let provider_config = provider_config.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if replace_api_key {
        // 使用配置的 API Key 替换请求头中的 API key
        api_key = provider_config.api_key.clone()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let api_url = provider_config.api_url.clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("📥 OpenAI request: model={}, url={}", req.model, api_url);

    let retry = provider_config.retry.clone();
    match provider_config.r#type {
        Provider::OpenAI => straight(state, req, api_url, api_key, retry.as_ref()).await,
        Provider::Anthropic => into_anthropic(state, req, api_url, api_key, retry.as_ref()).await,
        Provider::Gemini => into_gemini(state, req, api_url, api_key, retry.as_ref()).await,
    }
}

pub fn models(state: &AdaptState) -> OpenAIModelList {
    let providers = state.config.providers
        .iter()
        .filter(|p| p.enabled && p.r#type == Provider::OpenAI)
        .map(|p| p.key.clone())
        .collect::<Vec<_>>();
    let data = state.config.models
        .iter()
        .filter(|m| m.provider.as_ref()
            .map(|p| providers.contains(p))
            .unwrap_or(true)
        )
        .map(|m| OpenAIModelInfo {
            id: m.model.clone(),
            object: "model".into(),
            created: 0,
            owned_by: m.provider.clone().unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    OpenAIModelList { object: "list".into(), data }
}

// ============ OpenAI → OpenAI 直通 ============

async fn straight(
    state: Arc<AdaptState>,
    req: OpenAIChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    let is_streaming = req.stream.unwrap_or(false);
    println!("⚡ OpenAI passthrough (stream={})", is_streaming);

    let api_url = api_url
        .join("/v1/chat/completions")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, || 
        state.client
            .post(api_url.clone())
            .header("authorization", format!("Bearer {}", &api_key))
            .header("content-type", "application/json")
            .json(&req)
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        if is_streaming {
            let stream = response.bytes_stream();
            let body = Body::from_stream(stream);
            Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
        } else {
            let resp = response
                .json::<OpenAIChatResponse>()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            state.stats.record(&usage::extract(&resp));
            Ok(Json(resp).into_response())
        }
    }
}

// ============ OpenAI → Anthropic ============

async fn into_anthropic(
    state: Arc<AdaptState>,
    req: OpenAIChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    let is_streaming = req.stream.unwrap_or(false);
    println!("🔄 OpenAI → Anthropic (stream={})", is_streaming);

    let api_url = api_url
        .join("/v1/messages")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, || 
        state.client
            .post(api_url.clone())
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&AnthropicChatRequest::from(&req))
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        if is_streaming {
            anthropic::from_anthropic_streaming(response).await
        } else {
            let (resp, usage) = anthropic::from_anthropic_response(response).await?;
            state.stats.record(&usage);
            Ok(resp)
        }
    }
}

// ============ OpenAI → Gemini ============

async fn into_gemini(
    state: Arc<AdaptState>,
    req: OpenAIChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    let is_streaming = req.stream.unwrap_or(false);
    println!("🔄 OpenAI → Gemini (stream={})", is_streaming);

    // 转换请求: OpenAI -> Gemini
    let gemini_req = GeminiChatRequest::from(&req);

    // 构建URL，根据是否流式选择不同端点
    let endpoint = if is_streaming {
        format!("/v1beta/models/{}:streamGenerateContent?key={}&alt=sse", req.model, api_key)
    } else {
        format!("/v1beta/models/{}:generateContent?key={}", req.model, api_key)
    };

    let full_url = api_url
        .join(&endpoint)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, || 
        state.client
            .post(full_url.clone())
            .header("content-type", "application/json")
            .json(&gemini_req)
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        if is_streaming {
            gemini::from_gemini_streaming(response).await
        } else {
            let (resp, usage) = gemini::from_gemini_response(response).await?;
            state.stats.record(&usage);
            Ok(resp)
        }
    }
}