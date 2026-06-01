use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    Json, response::Response, http::StatusCode, body::Bytes,
    extract::{State, Path, Query}, response::IntoResponse,
};
use url::Url;
use aidapter::{
    Provider,
    gemini::prefix::{
        GeminiChatRequest, GeminiChatResponse,
        GeminiModelList, GeminiModelInfo,
        GeminiEmbedRequest, GeminiBatchEmbedRequest,
    },
    openai::prefix::OpenAIChatRequest,
    anthropic::prefix::AnthropicChatRequest,
};

use datum::{AdaptState, RetryConfig};

use crate::{retry, usage};

pub mod anthropic;
pub mod openai;

// ============ Gemini 路由处理器 ============

/// 解析 path 参数来提取 model 和 method
/// 格式：
/// {model}:generateContent 或 {model}:streamGenerateContent
/// {model}:batchEmbedContent 或 {model}:embedContent
pub async fn handler(
    State(state): State<Arc<AdaptState>>,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<Response, StatusCode> {
    // 解析路径：model:method
    let (mut model, method) = match path.rsplit_once(':') {
        Some((m, method)) => (m.to_string(), method),
        None => return Err(StatusCode::BAD_REQUEST),
    };
    let mut api_key = query.get("key").unwrap_or(&String::new()).to_string();
    let mut provider_config = None;
    let mut replace_api_key = api_key.is_empty();
    // 查找模型配置
    if let Some(model_config) = state.config.find_model(&model) {
        // BYOK 表示使用用户携带的 API Key，将不进行替换
        replace_api_key = !model_config.byokey;
        // 使用配置的模型名称替换请求中的模型名称
        if let Some(remodel) = &model_config.remodel {
            model = remodel.clone();
        }
        // 如果模型有供应商配置
        if let Some(provider) = &model_config.provider {
            provider_config = state.config.find_provider(provider)
        }
    }
    if provider_config.is_none() {
        provider_config = state.config.give_provider(Provider::Gemini);
    }
    let provider_config = provider_config.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if replace_api_key {
        // 使用配置的 API Key 替换请求头中的 API key
        api_key = provider_config.api_key.clone()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let api_url = provider_config.api_url.clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("📥 Gemini request: model={}, method={}, url={}", model, method, api_url);

    let retry = provider_config.retry.clone();
    match method {
        "generateContent" => {
            // Gemini API 端点: POST /v1beta/models/{model}:generateContent
            let req: GeminiChatRequest = serde_json::from_slice(&body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            match provider_config.r#type {
                Provider::OpenAI =>  into_openai(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::Anthropic => into_anthropic(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::Gemini => straight(state, model, req, api_url, api_key, retry.as_ref()).await,
            }
        },
        "streamGenerateContent" => {
            // Gemini API 流式端点: POST /v1beta/models/{model}:streamGenerateContent
            let req: GeminiChatRequest = serde_json::from_slice(&body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            match provider_config.r#type {
                Provider::OpenAI =>  streaming::into_openai(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::Anthropic => streaming::into_anthropic(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::Gemini => streaming::straight(state, model, req, api_url, api_key, retry.as_ref()).await,
            }
        },
        "embedContent" => {
            // Gemini API 嵌入端点: POST /v1beta/models/{model}:embedContent
            let req: GeminiEmbedRequest = serde_json::from_slice(&body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            match provider_config.r#type {
                Provider::Gemini => embedding::straight(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::OpenAI => embedding::into_openai(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::Anthropic => Err(StatusCode::BAD_REQUEST), // Anthropic 不支持 Embedding
            }
        },
        "batchEmbedContent" => {
            // Gemini API 批量嵌入端点: POST /v1beta/models/{model}:batchEmbedContent
            let req: GeminiBatchEmbedRequest = serde_json::from_slice(&body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            match provider_config.r#type {
                Provider::Gemini => embedding::straight_batch(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::OpenAI => embedding::into_openai_batch(state, model, req, api_url, api_key, retry.as_ref()).await,
                Provider::Anthropic => Err(StatusCode::BAD_REQUEST), // Anthropic 不支持 Batch Embedding
            }
        },
        _ => Err(StatusCode::NOT_FOUND),
    }
}

pub fn models(state: &AdaptState) -> GeminiModelList {
    let models = state.config.models
        .iter()
        .map(|m| GeminiModelInfo {
            name: format!("models/{}", m.model),
            display_name: m.name.clone().unwrap_or_else(|| m.model.clone()),
            description: m.name.clone().unwrap_or_else(|| m.model.clone()),
            supported_generation_methods: vec![
                "generateContent".into(),
                "streamGenerateContent".into(),
                "embedContent".into(),
                "batchEmbedContent".into(),
            ],
            version: "v1".to_string(),
            base_model_id: Some(m.model.clone()),
            input_token_limit: 0,
            output_token_limit: 0,
            temperature: None,
            top_p: None,
            top_k: None,
        })
        .collect::<Vec<_>>();
    GeminiModelList { models, next_page_token: None }
}

// ============ Gemini → Gemini 直通 ============

async fn straight(
    state: Arc<AdaptState>,
    model: String,
    req: GeminiChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    println!("⚡ Gemini passthrough");

    let api_url = api_url
        .join(&format!("/v1beta/models/{}:generateContent?key={}", model, api_key))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, || 
        state.client
            .post(api_url.clone())
            .header("content-type", "application/json")
            .json(&req)
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        let resp = response
            .json::<GeminiChatResponse>()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        state.stats.record(&usage::extract(&resp));
        Ok(Json(resp).into_response())
    }

}

// ============ Gemini → OpenAI ============

async fn into_openai(
    state: Arc<AdaptState>,
    model: String,
    req: GeminiChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    println!("🔄 Gemini → OpenAI");

    // 转换请求: Gemini -> OpenAI
    let mut openai_req = OpenAIChatRequest::from(&req);
    openai_req.model = model;

    let api_url = api_url
        .join("/v1/chat/completions")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, || 
        state.client
            .post(api_url.clone())
            .header("authorization", format!("Bearer {}", &api_key))
            .header("content-type", "application/json")
            .json(&openai_req)
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        let (resp, usage) = openai::from_openai_response(response).await?;
        state.stats.record(&usage);
        Ok(resp)
    }
}

// ============ Gemini → Anthropic ============

async fn into_anthropic(
    state: Arc<AdaptState>,
    model: String,
    req: GeminiChatRequest,
    api_url: Url,
    api_key: String,
    retry: Option<&RetryConfig>,
) -> Result<Response, StatusCode> {
    println!("🔄 Gemini → Anthropic");

    // 转换请求: Gemini -> Anthropic
    let mut anthropic_req = AnthropicChatRequest::from(&req);
    anthropic_req.model = model;

    let api_url = api_url
        .join("/v1/messages")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = retry::dispatch(retry, || 
        state.client
            .post(api_url.clone())
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&anthropic_req)
    ).await?;

    if !response.status().is_success() {
        Err(StatusCode::BAD_GATEWAY)
    } else {
        let (resp, usage) = anthropic::from_anthropic_response(response).await?;
        state.stats.record(&usage);
        Ok(resp)
    }

}

pub mod streaming {
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

    use datum::{AdaptState, RetryConfig};

    use crate::retry;
    use super::{openai, anthropic};

    // ============ Gemini → Gemini 直通Stream ============

    pub async fn straight(
        state: Arc<AdaptState>,
        model: String,
        req: GeminiChatRequest,
        api_url: Url,
        api_key: String,
        retry: Option<&RetryConfig>,
    ) -> Result<Response, StatusCode> {
        println!("⚡ Gemini streaming passthrough");

        let api_url = api_url
            .join(&format!("/v1beta/models/{}:streamGenerateContent?key={}&alt=sse", model, api_key))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = retry::dispatch(retry, ||
            state.client
                .post(api_url.clone())
                .header("content-type", "application/json")
                .json(&req)
        ).await?;

        if !response.status().is_success() {
            Err(StatusCode::BAD_GATEWAY)
        } else {
            let stream = response.bytes_stream();
            let body = Body::from_stream(stream);
            Ok((StatusCode::OK, [("content-type", "text/event-stream")], body).into_response())
        }

    }

    // ============ Gemini → OpenAI Stream ============

    pub async fn into_openai(
        state: Arc<AdaptState>,
        model: String,
        req: GeminiChatRequest,
        api_url: Url,
        api_key: String,
        retry: Option<&RetryConfig>,
    ) -> Result<Response, StatusCode> {
        println!("🔄 Gemini → OpenAI (streaming)");

        // 转换请求: Gemini -> OpenAI，并启用流式
        let mut openai_req = OpenAIChatRequest::from(&req);
        openai_req.model = model;
        openai_req.stream = Some(true);

        let api_url = api_url
            .join("/v1/chat/completions")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = retry::dispatch(retry, ||
            state.client
                .post(api_url.clone())
                .header("authorization", format!("Bearer {}", &api_key))
                .header("content-type", "application/json")
                .json(&openai_req)
        ).await?;

        if !response.status().is_success() {
            Err(StatusCode::BAD_GATEWAY)
        } else {
            openai::from_openai_streaming(response).await
        }
    }

    // ============ Gemini → Anthropic Stream ============

    pub async fn into_anthropic(
        state: Arc<AdaptState>,
        model: String,
        req: GeminiChatRequest,
        api_url: Url,
        api_key: String,
        retry: Option<&RetryConfig>,
    ) -> Result<Response, StatusCode> {
        println!("🔄 Gemini → Anthropic (streaming)");

        // 转换请求: Gemini -> Anthropic，并启用流式
        let mut anthropic_req = AnthropicChatRequest::from(&req);
        anthropic_req.model = model;
        anthropic_req.stream = Some(true);

        let api_url = api_url
            .join("/v1/messages")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = retry::dispatch(retry, || 
            state.client
                .post(api_url.clone())
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&anthropic_req)
        ).await?;

        if !response.status().is_success() {
            Err(StatusCode::BAD_GATEWAY)
        } else {
            anthropic::from_anthropic_streaming(response).await
        }
    }
}

pub mod embedding {
    use std::sync::Arc;
    use axum::{
        response::Response, http::StatusCode,
        Json, response::IntoResponse,
    };
    use url::Url;
    use aidapter::{
        gemini::prefix::{
            GeminiEmbedRequest, GeminiEmbedResponse,
            GeminiBatchEmbedRequest, GeminiBatchEmbedResponse,
        },
        openai::prefix::{
            OpenAIEmbedRequest, OpenAIEmbedResponse,
        },
    };
    use datum::{AdaptState, RetryConfig, TokenUsage};
    use crate::retry;

    // ============ Gemini → Gemini 直通 (单条) ============

    pub async fn straight(
        state: Arc<AdaptState>,
        model: String,
        req: GeminiEmbedRequest,
        api_url: Url,
        api_key: String,
        retry: Option<&RetryConfig>,
    ) -> Result<Response, StatusCode> {
        println!("⚡ Gemini embedding passthrough");

        let api_url = api_url
            .join(&format!("/v1beta/models/{}:embedContent?key={}", model, api_key))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = retry::dispatch(retry, ||
            state.client
                .post(api_url.clone())
                .header("content-type", "application/json")
                .json(&req)
        ).await?;

        if !response.status().is_success() {
            Err(StatusCode::BAD_GATEWAY)
        } else {
            let resp = response
                .json::<GeminiEmbedResponse>()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            state.stats.record(&TokenUsage {
                requests: 1,
                tokens: 0,
                input_tokens: 0,
                output_tokens: 0,
            });
            Ok(Json(resp).into_response())
        }
    }

    // ============ Gemini → Gemini 直通 (批量) ============

    pub async fn straight_batch(
        state: Arc<AdaptState>,
        model: String,
        req: GeminiBatchEmbedRequest,
        api_url: Url,
        api_key: String,
        retry: Option<&RetryConfig>,
    ) -> Result<Response, StatusCode> {
        println!("⚡ Gemini batch embedding passthrough");

        let api_url = api_url
            .join(&format!("/v1beta/models/{}:batchEmbedContent?key={}", model, api_key))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = retry::dispatch(retry, ||
            state.client
                .post(api_url.clone())
                .header("content-type", "application/json")
                .json(&req)
        ).await?;

        if !response.status().is_success() {
            Err(StatusCode::BAD_GATEWAY)
        } else {
            let resp = response
                .json::<GeminiBatchEmbedResponse>()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            state.stats.record(&TokenUsage {
                requests: 1,
                tokens: 0,
                input_tokens: 0,
                output_tokens: 0,
            });
            Ok(Json(resp).into_response())
        }
    }

    // ============ Gemini → OpenAI 转换 (单条) ============

    pub async fn into_openai(
        state: Arc<AdaptState>,
        model: String,
        req: GeminiEmbedRequest,
        api_url: Url,
        api_key: String,
        retry: Option<&RetryConfig>,
    ) -> Result<Response, StatusCode> {
        println!("🔄 Gemini embedding → OpenAI");

        // 转换请求: GeminiEmbedRequest → OpenAIEmbedRequest
        let mut openai_req = OpenAIEmbedRequest::from(&req);
        openai_req.model = model.clone();

        let api_url = api_url
            .join("/v1/embeddings")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = retry::dispatch(retry, ||
            state.client
                .post(api_url.clone())
                .header("authorization", format!("Bearer {}", &api_key))
                .header("content-type", "application/json")
                .json(&openai_req)
        ).await?;

        if !response.status().is_success() {
            Err(StatusCode::BAD_GATEWAY)
        } else {
            let openai_resp = response
                .json::<OpenAIEmbedResponse>()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            state.stats.record(&TokenUsage {
                requests: 1,
                tokens: openai_resp.usage.total_tokens as u64,
                input_tokens: openai_resp.usage.prompt_tokens as u64,
                output_tokens: 0,
            });
            // 转换响应: OpenAIEmbedResponse → GeminiEmbedResponse
            let gemini_resp = GeminiEmbedResponse::from(&openai_resp);
            Ok(Json(gemini_resp).into_response())
        }
    }

    // ============ Gemini → OpenAI 转换 (批量) ============

    pub async fn into_openai_batch(
        state: Arc<AdaptState>,
        model: String,
        req: GeminiBatchEmbedRequest,
        api_url: Url,
        api_key: String,
        retry: Option<&RetryConfig>,
    ) -> Result<Response, StatusCode> {
        println!("🔄 Gemini batch embedding → OpenAI");

        // 转换请求: GeminiBatchEmbedRequest → OpenAIEmbedRequest
        let mut openai_req = OpenAIEmbedRequest::from(&req);
        openai_req.model = model.clone();

        let api_url = api_url
            .join("/v1/embeddings")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = retry::dispatch(retry, ||
            state.client
                .post(api_url.clone())
                .header("authorization", format!("Bearer {}", &api_key))
                .header("content-type", "application/json")
                .json(&openai_req)
        ).await?;

        if !response.status().is_success() {
            Err(StatusCode::BAD_GATEWAY)
        } else {
            let openai_resp = response
                .json::<OpenAIEmbedResponse>()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            state.stats.record(&TokenUsage {
                requests: 1,
                tokens: openai_resp.usage.total_tokens as u64,
                input_tokens: openai_resp.usage.prompt_tokens as u64,
                output_tokens: 0,
            });
            // 转换响应: OpenAIEmbedResponse → GeminiBatchEmbedResponse
            let gemini_resp = GeminiBatchEmbedResponse::from(&openai_resp);
            Ok(Json(gemini_resp).into_response())
        }
    }

}