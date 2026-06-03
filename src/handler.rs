use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    Json, extract::{State, Path, OriginalUri},
    http::HeaderMap, http::StatusCode,
    response::{IntoResponse, Response},
};
use anyhow::Result;

use datum::{record::TokenUsage, session::SessionSnippet};
use adapt::{AdaptState, anthropic, gemini, openai};
use agent::AgentState;

// 统计查询端点
pub async fn query_stats(
    State(state): State<Arc<AdaptState>>
) -> Json<TokenUsage> {
    Json(state.stats.snapshot())
}

// 模型列表端点
pub async fn list_models(
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

pub async fn list_sessions(
    State(state): State<Arc<AgentState>>
) -> Result<Response, StatusCode> {
    let sessions = state.sessions.list_sessions()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sessions).into_response())
}

pub async fn create_session(
    State(state): State<Arc<AgentState>>,
    Json(mut snippet): Json<SessionSnippet>,
) -> Result<Response, StatusCode> {
    snippet.id = None;
    let session = state.sessions.gain_session(snippet)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(session).into_response())
}

pub async fn remove_session(
    State(state): State<Arc<AgentState>>,
    Path(sid): Path<String>,
) -> Result<Response, StatusCode> {
    state.sessions.delete_session(&sid)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(sid.into_response())
}
