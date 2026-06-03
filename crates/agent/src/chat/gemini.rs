use std::sync::Arc;
use std::collections::HashMap;
use std::convert::Infallible;
use axum::{
    Json, body::Bytes,
    extract::{State, Path, Query},
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
use aidapter::gemini::prefix::{
    GeminiChatRequest, GeminiChatResponse, GeminiContent, GeminiPart, GeminiRole,
    GeminiTool, GeminiFunctionDeclaration, GeminiFunctionResponse, GeminiCandidate,
    GeminiFinishReason,
};
use adapt::gemini;
use datum::session::SessionSnippet;
use crate::AgentState;
use super::{deser_resp, bytes_resp, session_id, MAX_RUNNING_ROUND};

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AgentState>>,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<GeminiChatRequest>,
) -> Result<Response, StatusCode> {
    let (model, method) = path.rsplit_once(':').unwrap_or((&path, "generateContent"));
    let streaming = method == "streamGenerateContent";
    let model = model.to_string();

    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(32);
    tokio::spawn(async move {
        let r = running(headers, state, req, path, query, streaming, &model, &tx).await;
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
        Ok(Json(GeminiChatResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContent {
                    role: Some(GeminiRole::Model),
                    parts: vec![GeminiPart::Text(text)]
                },
                finish_reason: Some(GeminiFinishReason::Stop),
                safety_ratings: None, citation_metadata: None,
                token_count: None, grounding_attributions: None, index: Some(0),
            }],
            prompt_feedback: None, usage_metadata: None, model_version: None,
        }).into_response())
    }
}

async fn running(
    headers: HeaderMap,
    state: Arc<AgentState>,
    mut req: GeminiChatRequest,
    path: String,
    query: HashMap<String, String>,
    streaming: bool,
    model: &str,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<()> {
    let talk = req.contents.last()
        .and_then(|c| c.parts.iter().filter_map(|p| match p {
            GeminiPart::Text(s) => Some(s.clone()), _ => None
        }).next()).unwrap_or_default();

    let _ = tx.send(Ok(String::new())).await;
    let mut session = state.sessions.gain_session(SessionSnippet {
        id: session_id(&headers),
        title: talk.clone(),
        model: model.to_string(),
        provider: "gemini".to_string(),
    })?;
    let _ = state.sessions.create_message(&mut session, &talk, "user")?;

    let tools: Vec<GeminiTool> = vec![GeminiTool {
        function_declarations: Some(state.toolctx.tools.iter().map(|td| GeminiFunctionDeclaration {
            name: td.name.into(), description: Some(td.description.into()), parameters: None,
        }).collect()),
        code_execution: None,
    }];
    let mut msgs: Vec<GeminiContent> = session.messages.iter().map(|m| m.into()).collect();
    req.tools = Some(tools);

    for _ in 1..=MAX_RUNNING_ROUND {
        req.contents = msgs.clone();

        let body = Bytes::from(serde_json::to_vec(&req).unwrap_or_default());
        let resp = gemini::handler(
            State(state.adapter.clone()),
            Path(path.clone()),
            Query(query.clone()),
            body,
        ).await.map_err(|e| anyhow!(e))?;

        if !resp.status().is_success() {
            return Err(anyhow!("HTTP {}", resp.status()));
        }

        let (text, calls) = if streaming {
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
        msgs.push(GeminiContent {
            role: Some(GeminiRole::Model),
            parts: calls.clone(),
        });

        let mut tlparts = Vec::new();
        for part in &calls {
            match part {
                GeminiPart::FunctionCall(fc) => {
                    let result = state.toolctx.execute(&fc.name, &fc.args.as_ref().unwrap_or_default());
                    let content = if result.success { result.content.clone() } else { format!("ERROR: {}", result.content) };
                    tlparts.push(GeminiPart::FunctionResponse(GeminiFunctionResponse {
                        name: fc.name.clone(),
                        response: serde_json::json!({ "result": &content }),
                    }));
                    let _ = state.sessions.create_message(&mut session, &content, "tool")?;
                }
                _ => continue,
            }
        }
        msgs.push(GeminiContent {
            role: Some(GeminiRole::User),
            parts: tlparts
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
) -> Result<(String, Vec<GeminiPart>)> {
    let resp: GeminiChatResponse = deser_resp(resp).await?;
    let mut parts = vec![];
    let mut content = String::new();
    for c in &resp.candidates {
        for part in &c.content.parts {
            match part {
                GeminiPart::Text(text) => content.push_str(text),
                _ => parts.push(part.clone()),
            }
        }
    }
    let _ = tx.send(Ok(content.clone())).await;
    let _ = tx.send(Ok("[DONE]".into())).await;
    Ok((content, parts))
}

// SSE 流式解析（EventStream）
async fn sse_parse(
    resp: Response, tx: &Sender<Result<String, Infallible>>,
) -> Result<(String, Vec<GeminiPart>)> {
    let mut events = bytes_resp(resp).await;
    let mut fulltxt = String::new();
    let mut calls: Vec<GeminiPart> = Vec::new();

    while let Some(ev) = events.next().await {
        let ev = match ev {
            Ok(e) => e,
            Err(e) => return Err(anyhow!(e)),
        };
        if ev.data == "[DONE]" || ev.data.is_empty() {
            let _ = tx.send(Ok(ev.data)).await;
            continue;
        }

        let v: GeminiChatResponse = serde_json::from_str(&ev.data)?;
        for c in &v.candidates {
            for part in &c.content.parts {
                match part {
                    GeminiPart::Text(text) => fulltxt.push_str(text),
                    _ => calls.push(part.clone()),
                }
            }
        }
    }

    Ok((fulltxt, calls))
}
