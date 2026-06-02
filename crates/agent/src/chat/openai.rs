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
use tokio::sync::{mpsc, mpsc::Sender};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use aidapter::openai::prefix::{
    OpenAIChatRequest, OpenAIChatResponse, OpenAIMessage, OpenAIMessageContent,
    OpenAIToolCall, OpenAITool, OpenAIContentPart, OpenAIChoice, OpenAIUsage
};
use adapt::openai;
use crate::AgentState;
use super::{deser_resp, session_id, MAX_RUNNING_ROUND};

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AgentState>>,
    Json(req): Json<OpenAIChatRequest>,
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
        Ok(Json(OpenAIChatResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            model: model,
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
                total_tokens: 0,
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
    let sid = session_id(&headers);

    let model = req.model.clone();
    let talk = req.messages.last()
        .and_then(|m| match m {
            OpenAIMessage::User { content, .. } => {
                match content {
                    OpenAIMessageContent::String(s) => Some(s.clone()),
                    OpenAIMessageContent::Array(c) => {
                        let c= c.iter().map(|s| match s { 
                            OpenAIContentPart::Text { text } => text.clone(),
                            _ => String::new() 
                        }).collect::<Vec<_>>().join("\n");
                        Some(c)
                    }
                }
            }, _ => None,
        }).unwrap_or_default();

    let _ = tx.send(Ok(String::new())).await;
    let mut session = state.sessions.gain_session(&sid, &talk, "openai", &model)?;
    let _ = state.sessions.create_message(&mut session, &talk, "user")?;
    let tools: Vec<OpenAITool> = state.toolctx.tools.iter().map(|td| td.into()).collect();
    let mut msgs: Vec<OpenAIMessage> = session.messages.iter().map(|m| m.into()).collect();
    req.tools = Some(tools);
    req.stream = Some(false);

    for _ in 1..=MAX_RUNNING_ROUND {
        req.messages = msgs.clone();

        let resp = openai::handler(
            headers.clone(), 
            State(state.adapter.clone()), 
            Json(req.clone())
        ).await.map_err(|e| anyhow!(e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(format!("HTTP {}", resp.status())));
        }

        let resp: OpenAIChatResponse = deser_resp(resp).await?;
        let calls: Vec<OpenAIToolCall> = resp.choices.iter()
            .filter_map(|c| match &c.message { 
                OpenAIMessage::Assistant { tool_calls, .. } => tool_calls.clone(),
                _ => None 
            })
            .flatten().collect();

        if calls.is_empty() {
            let content = resp.choices.first()
                .and_then(|c| match &c.message {
                    OpenAIMessage::Assistant { content, .. } => content.clone(),
                    _ => None 
                })
                .map(|c| match c {
                    OpenAIMessageContent::String(s) => s.clone(),
                    _ => String::new() 
                }).unwrap_or_default();
            let _ = state.sessions.create_message(&mut session, &content, "assistant")?;
            session.update();
            let _ = state.sessions.save_metadata(&session);
            let _ = tx.send(Ok(content.into())).await;
            let _ = tx.send(Ok("[DONE]".into())).await;
            return Ok(());
        }

        let _ = tx.send(Ok(format!("tool:{}", calls.len()))).await;
        msgs.push(OpenAIMessage::Assistant {
            tool_calls: Some(calls.clone()),
            audio: None, content: None, name: None,
            function_call: None,  refusal: None,
        });
        for tc in &calls {
            let (tc_id, fn_name, fn_args) = match tc {
                OpenAIToolCall::Function { function, id } => {
                    (id, &function.name, &function.arguments)
                },
                _ => continue,
            };
            let args = serde_json::from_str(fn_args).unwrap_or_default();
            let result = state.toolctx.execute(fn_name, &args);
            let content = if result.success { result.content } else { format!("ERROR: {}", result.content) };
            msgs.push(OpenAIMessage::Tool {
                content: OpenAIMessageContent::String(content.clone()),
                tool_call_id: tc_id.clone(),
            });
            let _ = state.sessions.create_message(&mut session, &content, "tool")?;
        }
        session.update();
        let _ = state.sessions.save_metadata(&session);
    }
    Err(anyhow!("max rounds"))
}