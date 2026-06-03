use std::sync::Arc;
use axum::{Router, routing::{get, post, delete}};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tower_http::cors::{self, CorsLayer};

use datum::{
    config::Config, session::SessionStore,
    record::UsageStats,
};
use adapt::{AdaptState, anthropic, gemini, openai};
use agent::{AgentState, tools::ToolContext, chat};

mod handler;

#[tokio::main]
async fn main() {
    // 加载配置
    let config = Config::load();

    // 验证 API keys
    config.validate();

    let adapt_state = Arc::new(AdaptState {
        client: reqwest::Client::new(),
        config: config.clone(),
        stats: UsageStats::default(),
    });

    let mut toolctx = ToolContext::new(&config.agent.workspace);
    toolctx.filte(&config.agent.tools);

    let agent_state = Arc::new(AgentState {
        adapter: adapt_state.clone(),
        toolctx,
        sessions: SessionStore::new(&config.directory()),
    });

    let app = Router::new()
        // Anthropic 格式端点
        .route("/v1/messages", post(anthropic::handler))
        // OpenAI 格式端点
        .route("/v1/chat/completions", post(openai::handler))
        // OpenAI Embedding 端点
        .route("/v1/embeddings", post(openai::embedding))
        // Gemini 格式端点 (使用通配符捕获 model:method 部分)
        .route("/v1beta/models/{*path}", post(gemini::handler))
        // 模型列表，根据请求头识别 OpenAI / Anthropic 格式
        .route("/v1/models", get(handler::list_models))
        // 模型列表，根据请求路径识别 Gemini 格式
        .route("/v1beta/models", get(handler::list_models))
        // 运行状态查询
        .route("/api/stats", get(handler::query_stats))
        // 模型列表查询，通用格式
        .route("/api/models", get(handler::list_models))
        .with_state(adapt_state)
        // 会话列表查询
        .route("/api/sessions", get(handler::list_sessions))
        // 会话创建
        .route("/api/sessions", post(handler::create_session))
        // 会话删除
        .route("/api/sessions/{id}", delete(handler::remove_session))
        // Agent 端点（原生格式，会话管理 + 工具执行）
        .nest("/api/agent", chat::router())
        .with_state(agent_state)
        // 静态文件服务
        .fallback_service(ServeDir::new(&config.public))
        .layer(CorsLayer::new().allow_origin(cors::Any).allow_methods(cors::Any).allow_headers(cors::Any));

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect(&format!("Failed bind address to {}", addr));

    println!("🚀 Tri-directional streaming proxy on http://{}", addr);
    println!("📡 Adapter Routes:");
    println!("   - POST /v1/messages (Anthropic format)");
    println!("   - POST /v1/chat/completions (OpenAI format)");
    println!("   - POST /v1/embeddings (OpenAI embedding format)");
    println!("   - POST /v1beta/models/{{model}}:generateContent (Gemini format)");
    println!("   - POST /v1beta/models/{{model}}:streamGenerateContent (Gemini streaming)");
    println!("   - POST /v1beta/models/{{model}}:embedContent (Gemini embedding)");
    println!("   - POST /v1beta/models/{{model}}:batchEmbedContent (Gemini batch embedding)");
    println!("✨ Streaming support enabled for all routes");
    println!("📡 Static File Routes:");
    println!("   - GET  {{path}} (Static files from {})", config.public);
    println!("📡 Model List Routes:");
    println!("   - GET  /v1/models (OpenAI / Anthropic model list)");
    println!("   - GET  /v1beta/models (Gemini model list)");
    println!("📡 API Routes:");
    println!("   - GET  /api/stats (Usage statistics)");
    println!("   - GET  /api/models (Model list)");
    println!("   - GET  /api/sessions (Session list)");
    println!("📡 Chat Routes:");
    println!("   - POST /api/agent/v1/chat/completions (OpenAI chat)");
    println!("   - POST /api/agent/v1/messages (Anthropic chat)");
    println!("   - POST /api/agent/v1beta/models/{{model}}:generateContent (Gemini chat)");
    println!("   - POST /api/agent/v1beta/models/{{model}}:streamGenerateContent (Gemini streaming chat)");

    // 优雅关闭
    let signal = async {
        tokio::signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(signal)
        .await
        .unwrap();
}
