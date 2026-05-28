use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    Json, Router, extract::{State, OriginalUri},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use tokio::net::TcpListener;

use datum::{Config, AdaptState, TokenUsage, UsageStats};
use adapt::{anthropic, gemini, openai};

// 统计查询端点
async fn stats(State(state): State<Arc<AdaptState>>) -> Json<TokenUsage> {
    Json(state.stats.snapshot())
}

// 模型列表端点
async fn models(
    State(state): State<Arc<AdaptState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    if uri.path().ends_with("/api/models") {
        let providers: HashMap<String, String> = state.config.providers.iter()
            .filter(|p| p.enabled)
            .map(|p| (p.key.clone(), p.provider()))
            .collect();
        let mut models = state.config.models.clone();
        let models = models.iter_mut()
            .map(|m| {
                if let Some(p) = m.provider.as_ref() {
                    m.provider = providers.get(p).cloned();
                }
                m
            }).collect::<Vec<_>>();
        Json(models).into_response()
    } else if uri.path().ends_with("/v1beta/models") {
        Json(gemini::models(&state)).into_response()
    } else if headers.get("x-api-key").is_some() {
        Json(anthropic::models(&state)).into_response()
    } else {
        Json(openai::models(&state)).into_response()
    }
}

#[tokio::main]
async fn main() {
    // 加载配置
    let config = Config::load();

    // 验证 API keys
    config.validate();

    let state = Arc::new(AdaptState {
        client: reqwest::Client::new(),
        config: config.clone(),
        stats: UsageStats::default(),
    });

    let app = Router::new()
        // Anthropic 格式端点
        .route("/v1/messages", post(anthropic::handler))
        // OpenAI 格式端点
        .route("/v1/responses", post(openai::handler))
        .route("/v1/chat/completions", post(openai::handler))
        // Gemini 格式端点 (使用通配符捕获 model:method 部分)
        .route("/v1beta/models/{*path}", post(gemini::handler))
        // 模型列表，根据请求头识别 OpenAI / Anthropic 格式
        .route("/v1/models", get(models))
        // 模型列表，根据请求路径识别 Gemini 格式
        .route("/v1beta/models", get(models))
        // 运行状态查询
        .route("/api/stats", get(stats))
        // 模型列表查询，通用格式
        .route("/api/models", get(models))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect(&format!("Failed to bind to {}", addr));

    println!("🚀 Tri-directional streaming proxy on http://{}", addr);
    println!("📡 Adapter Routes:");
    println!("   - POST /v1/messages (Anthropic format)");
    println!("   - POST /v1/responses (OpenAI format)");
    println!("   - POST /v1/chat/completions (OpenAI format)");
    println!("   - POST /v1beta/models/{{model}}:generateContent (Gemini format)");
    println!("   - POST /v1beta/models/{{model}}:streamGenerateContent (Gemini streaming)");
    println!("✨ Streaming support enabled for all routes");
    println!("📡 Model List Routes:");
    println!("   - GET  /v1/models (OpenAI / Anthropic model list)");
    println!("   - GET  /v1beta/models (Gemini model list)");
    println!("📊 API Routes:");
    println!("   - GET  /api/stats (Usage statistics)");

    axum::serve(listener, app).await.unwrap();
}
