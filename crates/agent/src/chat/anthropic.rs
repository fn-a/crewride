use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    Json, extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse}
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
    AnthropicStreamChunk, AnthropicStreamEvent, AnthropicContentDelta,
};
use adapt::anthropic;
use datum::session::SessionSnippet;
use crate::AgentState;
use super::{deser_resp, bytes_resp, session_id, MAX_RUNNING_ROUND};

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AgentState>>,
    Json(req): Json<AnthropicChatRequest>,
) -> Result<Response, StatusCode> {
    let streaming = req.stream.unwrap_or(false);
    let model = req.model.clone();

    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(32);
    tokio::spawn(async move {
        let r = running(headers, state, req, &tx).await;
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
            if data.starts_with("tool:") { continue; }
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
    let talk = req.messages.last()
        .and_then(|m| match &m.content {
            AnthropicContent::Text(s) => Some(s.clone()),
            AnthropicContent::Parts(parts) => parts.iter()
                .filter_map(|p| match p {
                    AnthropicContentPart::Text { text } => Some(text.clone()),
                    _ => None
                }).next(),
        }).unwrap_or_default();

    let _ = tx.send(Ok(String::new())).await;
    let mut session = state.sessions.gain_session(SessionSnippet {
        id: session_id(&headers),
        title: talk.clone(),
        model: req.model.clone(),
        provider: "anthropic".to_string(),
    })?;
    let _ = state.sessions.create_message(&mut session, &talk, "user")?;

    let tools: Vec<AnthropicTool> = state.toolctx.tools.iter().map(|td| td.into()).collect();
    let mut msgs: Vec<AnthropicMessage> = session.messages.iter().map(|m| m.into()).collect();
    req.tools = Some(tools);

    for _ in 1..=MAX_RUNNING_ROUND {
        req.messages = msgs.clone();

        let resp = anthropic::handler(
            headers.clone(),
            State(state.adapter.clone()),
            Json(req.clone()),
        ).await.map_err(|e| anyhow!(e))?;

        if !resp.status().is_success() {
            return Err(anyhow!("HTTP {}", resp.status()));
        }

        let (text, calls) = if req.stream.unwrap_or(false) {
            sse_parse(resp, tx).await?
        } else {
            des_parse(resp, tx).await?
        };

        if calls.is_empty() {
            let _ = state.sessions.create_message(&mut session, &text, "assistant")?;
            session.update();
            let _ = state.sessions.save_metadata(&session);
            return Ok(());
        }

        let _ = tx.send(Ok(format!("tool:{}", calls.len()))).await;
        msgs.push(AnthropicMessage {
            role: AnthropicRole::Assistant,
            content: AnthropicContent::Parts(calls.clone()),
        });

        let mut results = Vec::new();
        for part in &calls {
            match part {
                AnthropicContentPart::ToolUse { id, name, input } => {
                    let result = state.toolctx.execute(name, input);
                    let content = if result.success { result.content } else { format!("ERROR: {}", result.content) };
                    let _ = state.sessions.create_message(&mut session, &content, "tool")?;
                    results.push(AnthropicContentPart::ToolResult {
                        tool_use_id: id.clone(), content, is_error: Some(!result.success),
                    });
                }
                _ => continue,
            }
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

// 非流式解析
async fn des_parse(
    resp: Response,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<(String, Vec<AnthropicContentPart>)> {
    let resp: AnthropicChatResponse = deser_resp(resp).await?;
    let mut parts = vec![];
    let mut content = String::new();
    for part in resp.content.iter() {
        match part {
            AnthropicContentPart::Text { text } => content.push_str(text),
            _ => parts.push(part.clone()),
        }
    }
    let _ = tx.send(Ok(content.clone())).await;
    let _ = tx.send(Ok("[DONE]".into())).await;
    Ok((content, parts))
}

// SSE 流式解析（EventStream）
async fn sse_parse(
    resp: Response,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<(String, Vec<AnthropicContentPart>)> {
    let mut events = bytes_resp(resp).await;
    let mut fulltxt = String::new();
    let mut tlparts: Vec<AnthropicContentPart> = Vec::new();

    while let Some(ev) = events.next().await {
        let ev = match ev {
            Ok(e) => e,
            Err(e) => return Err(anyhow!(e)),
        };
        if ev.data == "[DONE]" || ev.data.is_empty() {
            let _ = tx.send(Ok(ev.data)).await;
            continue;
        }

        let v: AnthropicStreamChunk = serde_json::from_str(&ev.data)?;

        let _ = tx.send(Ok(ev.data)).await;

        match v.event {
            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                match delta {
                    AnthropicContentDelta::TextDelta { text } => {
                        fulltxt.push_str(&text);
                    }
                    _ => continue,
                }
            }
            AnthropicStreamEvent::ContentBlockStart { content_block, .. } => {
                tlparts.push(content_block.clone());
            }
            _ => continue,
        }
    }

    Ok((fulltxt, tlparts))
}
