use std::sync::Arc;
use std::convert::Infallible;
use std::collections::HashMap;
use axum::{
    Json, extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse}
    },
};
use tokio::sync::{mpsc, mpsc::Sender};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use aidapter::openai::prefix::{
    OpenAIChatRequest, OpenAIChatResponse, OpenAIMessage, OpenAIMessageContent,
    OpenAIToolCall, OpenAITool, OpenAIContentPart, OpenAIChoice, OpenAIUsage,
    OpenAIFunctionCall, OpenAIStreamChunk,
};
use adapt::openai;
use datum::session::SessionSnippet;
use crate::AgentState;
use super::{deser_resp, bytes_resp, session_id, MAX_RUNNING_ROUND};

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AgentState>>,
    Json(req): Json<OpenAIChatRequest>,
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
        Ok(Json(OpenAIChatResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            model,
            choices: vec![OpenAIChoice {
                index: 0,
                finish_reason: Some("stop".into()),
                message: OpenAIMessage::Assistant {
                    content: Some(OpenAIMessageContent::String(text)),
                    name: None, refusal: None, tool_calls: None,
                    function_call: None, audio: None,
                },
            }],
            usage: OpenAIUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0
            },
            created: chrono::Utc::now().timestamp(),
            system_fingerprint: None,
        }).into_response())
    }
}

async fn running(
    headers: HeaderMap,
    state: Arc<AgentState>,
    mut req: OpenAIChatRequest,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<()> {
    let talk = req.messages.last()
        .and_then(|m| match m {
            OpenAIMessage::User { content, .. } => {
                 match content {
                    OpenAIMessageContent::String(s) => Some(s.clone()),
                    OpenAIMessageContent::Array(c) => {
                        let c = c.iter().map(|s| match s {
                            OpenAIContentPart::Text { text } => text.clone(),
                            _ => String::new()
                        }).collect::<Vec<_>>().join("\n");
                        Some(c)
                    }
                }
            }
            _ => None,
        }).unwrap_or_default();

    let _ = tx.send(Ok(String::new())).await;
    let mut session = state.sessions.gain_session(SessionSnippet {
        id: session_id(&headers),
        title: talk.clone(),
        model: req.model.clone(),
        provider: "openai".to_string(),
    })?;
    let _ = state.sessions.create_message(&mut session, &talk, "user")?;
    let tools: Vec<OpenAITool> = state.toolctx.tools.iter().map(|td| td.into()).collect();
    let mut msgs: Vec<OpenAIMessage> = session.messages.iter().map(|m| m.into()).collect();
    req.tools = Some(tools);

    for _ in 1..=MAX_RUNNING_ROUND {
        req.messages = msgs.clone();

        let resp = openai::handler(
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
            session.update(); let _ = state.sessions.save_metadata(&session);
            return Ok(());
        }

        let _ = tx.send(Ok(format!("tool:{}", calls.len()))).await;
        msgs.push(OpenAIMessage::Assistant {
            tool_calls: Some(calls.clone()),
            audio: None, content: None, name: None,
            function_call: None, refusal: None,
        });
        for tc in &calls {
            match tc {
                OpenAIToolCall::Function { function, id } => {
                    let args = serde_json::from_str(&function.arguments)?;
                    let result = state.toolctx.execute(&function.name, &args);
                    let content = if result.success { result.content } else { format!("ERROR: {}", result.content) };
                    let _ = state.sessions.create_message(&mut session, &content, "tool")?;
                    msgs.push(OpenAIMessage::Tool {
                        content: OpenAIMessageContent::String(content),
                        tool_call_id: id.clone(),
                    });
                }
                _ => continue,
            };
        }
        session.update();
        let _ = state.sessions.save_metadata(&session);
    }
    Err(anyhow!("max rounds"))
}

// 非流式解析
async fn des_parse(
    resp: Response,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<(String, Vec<OpenAIToolCall>)> {
    let resp: OpenAIChatResponse = deser_resp(resp).await?;
    let calls: Vec<OpenAIToolCall> = resp.choices.iter()
        .filter_map(|c| match &c.message { 
            OpenAIMessage::Assistant { tool_calls, .. } => tool_calls.clone(), 
            _ => None 
        }).flatten().collect();
    let content = resp.choices.first()
        .and_then(|c| match &c.message { 
            OpenAIMessage::Assistant { content, .. } => content.clone(), 
            _ => None 
        })
        .map(|c| match c { 
            OpenAIMessageContent::String(s) => s, 
            _ => String::new() 
        }).unwrap_or_default();
    let _ = tx.send(Ok(content.clone())).await;
    let _ = tx.send(Ok("[DONE]".into())).await;
    Ok((content, calls))
}

// SSE 流式解析（EventStream）
async fn sse_parse(
    resp: Response,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<(String, Vec<OpenAIToolCall>)> {
    let mut events = bytes_resp(resp).await;
    let mut fulltxt = String::new();
    let mut callms: HashMap<String, OpenAIFunctionCall> = HashMap::new();
    let mut calls: Vec<OpenAIToolCall> = Vec::new();

    while let Some(ev) = events.next().await {
        let ev = match ev {
            Ok(e) => e,
            Err(e) => return Err(anyhow!(e)),
        };
        if ev.data == "[DONE]" || ev.data.is_empty() {
            let _ = tx.send(Ok(ev.data)).await;
            continue;
        }
        
        let v: OpenAIStreamChunk = serde_json::from_str(&ev.data)?;
        
        let _ = tx.send(Ok(ev.data)).await;

        if let Some(choice) = v.choices.first() {
            // Text delta
            if let Some(content) = choice.delta.content.as_ref() {
                fulltxt.push_str(content);
            }
            // Tool call deltas
            if let Some(arr) = choice.delta.tool_calls.as_ref() {
                for tc in arr {
                    match tc {
                        OpenAIToolCall::Function { id, function } => {
                            let _ = callms.entry(id.clone()).and_modify(|v| {
                                v.arguments.push_str(function.arguments.as_str());
                            }).or_insert(function.clone());
                        }
                        _ => calls.push(tc.clone()),
                    }
                }
            }
        }
    }
    
    for (id, function) in callms.into_iter() {
        calls.push(OpenAIToolCall::Function { id: id.to_string(), function });
    }

    Ok((fulltxt, calls))
}
