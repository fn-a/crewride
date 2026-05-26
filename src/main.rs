use axum::{
    Json, Router, extract::State,
    routing::{get, post}
};
use std::sync::Arc;

use datum::{Config, AdaptState, TokenUsage, UsageStats};
use adapt::{anthropic, gemini, openai};

// 统计查询端点
async fn stats(State(state): State<Arc<AdaptState>>) -> Json<TokenUsage> {
    Json(state.stats.snapshot())
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
        .route("/v1/chat/completions", post(openai::handler))
        // Gemini 格式端点 (使用通配符捕获 model:method 部分)
        .route("/v1beta/models/{*path}", post(gemini::handler))
        // 运行状态查询
        .route("/api/stats", get(stats))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect(&format!("Failed to bind to {}", addr));

    println!("🚀 Tri-directional streaming proxy on http://{}", addr);
    println!("📡 Adapter Routes:");
    println!("   - POST /v1/messages (Anthropic format)");
    println!("   - POST /v1/chat/completions (OpenAI format)");
    println!("   - POST /v1beta/models/{{model}}:generateContent (Gemini format)");
    println!("   - POST /v1beta/models/{{model}}:streamGenerateContent (Gemini streaming)");
    println!("✨ Streaming support enabled for all routes");
    println!("📊 API Routes:");
    println!("   - GET  /api/stats (Usage statistics)");

    axum::serve(listener, app).await.unwrap();
}
