use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    Json, extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
};
use uuid::Uuid;
use tokio::sync::{mpsc, mpsc::Sender};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use aidapter::anthropic::prefix::{
    AnthropicChatRequest, AnthropicChatResponse, AnthropicMessage, AnthropicRole,
    AnthropicContent, AnthropicContentPart, AnthropicTool, AnthropicUsage,
};
use adapt::anthropic;
use crate::AgentState;
use super::{deser_resp, session_id, MAX_RUNNING_ROUND};

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AgentState>>,
    Json(req): Json<AnthropicChatRequest>,
) -> Result<Response, StatusCode> {
    let streaming = req.stream.unwrap_or(false);
    let model = req.model.clone();

    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(32);
    tokio::spawn(async move {
        let r = running(headers, state.clone(), req, &tx).await;
        if let Err(e) = r {
            let _ = tx.send(Ok(format!("error:{}", e))).await;
        }
    });

    if streaming {
        let stream = ReceiverStream::new(rx)
            .map(|r| r.map(|d| Event::default().data(d)));
        Ok(Sse::new(stream).into_response())
    } else {
        let mut rx = ReceiverStream::new(rx);
        let mut text = String::new();
        while let Some(Ok(data)) = rx.next().await {
            if data == "[DONE]" { break; }
            if data.starts_with("error:") {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            text.push_str(&data);
        }
        Ok(Json(AnthropicChatResponse {
            id: format!("msg_{}", Uuid::new_v4()),
            r#type: "message".into(),
            role: AnthropicRole::Assistant,
            content: vec![AnthropicContentPart::Text { text }],
            model,
            stop_reason: Some("end_turn".into()),
            stop_sequence: None,
            usage: AnthropicUsage {
                input_tokens: 0, cache_read_input_tokens: None,
                cache_creation_input_tokens: None, output_tokens: 0,
            },
        }).into_response())
    }
}

async fn running(
    headers: HeaderMap,
    state: Arc<AgentState>,
    mut req: AnthropicChatRequest,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<()> {
    let sid = session_id(&headers);
    let model = req.model.clone();

    let user_text = req.messages.last()
        .and_then(|m| match &m.content {
            AnthropicContent::Text(s) => Some(s.clone()),
            AnthropicContent::Parts(parts) => parts.iter()
                .filter_map(|p| match p {
                    AnthropicContentPart::Text { text } => Some(text.clone()),
                    _ => None 
                }).next(),
        }).unwrap_or_default();

    let _ = tx.send(Ok(String::new())).await;
    let mut session = state.sessions.gain_session(&sid, &user_text, "anthropic", &model)?;
    let _ = state.sessions.create_message(&mut session, &user_text, "user")?;

    let tools: Vec<AnthropicTool> = state.toolctx.tools.iter().map(|td| td.into()).collect();
    let mut msgs: Vec<AnthropicMessage> = session.messages.iter().map(|m| m.into()).collect();
    req.tools = Some(tools);
    req.stream = Some(false);

    for _ in 1..=MAX_RUNNING_ROUND {
        req.messages = msgs.clone();

        let resp = anthropic::handler(
            headers.clone(),
            State(state.adapter.clone()),
            Json(req.clone()),
        ).await.map_err(|e| anyhow!(e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(format!("HTTP {}", resp.status())));
        }

        let resp: AnthropicChatResponse = deser_resp(resp).await?;
        let calls: Vec<(String, String, serde_json::Value)> = resp.content.iter()
            .filter_map(|cp| match cp {
                AnthropicContentPart::ToolUse { id, name, input } =>
                    Some((id.clone(), name.clone(), input.clone())),
                _ => None,
            }).collect();

        if calls.is_empty() {
            let content = resp.content.iter()
                .filter_map(|cp| match cp {
                    AnthropicContentPart::Text { text } => Some(text.clone()),
                    _ => None
                }).collect::<Vec<_>>().join("");
            let _ = state.sessions.create_message(&mut session, &content, "assistant")?;
            session.update();
            let _ = state.sessions.save_metadata(&session);
            let _ = tx.send(Ok(content)).await;
            let _ = tx.send(Ok("[DONE]".into())).await;
            return Ok(());
        }

        let _ = tx.send(Ok(format!("tool:{}", calls.len()))).await;
        msgs.push(AnthropicMessage {
            role: AnthropicRole::Assistant,
            content: AnthropicContent::Parts(resp.content.clone()),
        });

        let mut results = Vec::new();
        for (tc_id, fn_name, fn_input) in &calls {
            let result = state.toolctx.execute(fn_name, fn_input);
            let content = if result.success {
                result.content.clone()
            } else {
                format!("ERROR: {}", result.content)
            };
            results.push(AnthropicContentPart::ToolResult {
                tool_use_id: tc_id.clone(), content: content.clone(), is_error: Some(!result.success),
            });
            let _ = state.sessions.create_message(&mut session, &content, "tool")?;
        }
        msgs.push(AnthropicMessage {
            role: AnthropicRole::User,
            content: AnthropicContent::Parts(results),
        });
        session.update();
        let _ = state.sessions.save_metadata(&session);
    }
    Err(anyhow!("max rounds"))
}
