use std::sync::Arc;
use serde::de::Deserialize;
use axum::{
    Router, body::to_bytes,
    http::HeaderMap,
    response::Response,
    routing::post,
    body::BodyDataStream
};
use anyhow::Result;
use eventsource_stream::EventStream;
use crate::AgentState;

pub const MAX_RUNNING_ROUND: usize = 8;
pub const HEADER_SESSION_ID: &str = "x-session-id";

mod openai;
mod anthropic;
mod gemini;

pub fn router() -> Router<Arc<AgentState>> {
    Router::new()
        .route("/v1/chat/completions", post(openai::handler))
        .route("/v1/messages", post(anthropic::handler))
        .route("/v1beta/models/{*path}", post(gemini::handler))
}

pub async fn deser_resp<T>(resp: Response) -> Result<T> 
where for<'de> T: Deserialize<'de>
{
    let bytes = to_bytes(resp.into_body(), usize::MAX).await?;
    let result: T = serde_json::from_slice(&bytes)?;
    Ok(result)
}

pub async fn bytes_resp(resp: Response) -> EventStream<BodyDataStream> {
    let stream = resp.into_body().into_data_stream();
    let events = EventStream::new(stream);
    events
}

pub fn session_id(headers: &HeaderMap) -> Option<String> {
    headers.get(HEADER_SESSION_ID)
        .and_then(|v| v.to_str().ok().map(|s| s.to_string()))
}