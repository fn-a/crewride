use std::sync::Arc;
use std::collections::HashMap;
use std::convert::Infallible;
use axum::{
    Json, body::Bytes,
    extract::{State, Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
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
use crate::AgentState;
use super::{deser_resp, session_id, MAX_RUNNING_ROUND};

pub async fn handler(
    headers: HeaderMap,
    State(state): State<Arc<AgentState>>,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<GeminiChatRequest>,
) -> Result<Response, StatusCode> {
    let (model, _method) = path.rsplit_once(':').unwrap_or((&path, "generateContent"));
    let model = model.to_string();

    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(32);
    tokio::spawn(async move {
        let r = running(headers, state.clone(), req, query, &model, &tx).await;
        if let Err(e) = r {
            let _ = tx.send(Ok(format!("error:{}", e))).await;
        }
    });

    let mut rx = ReceiverStream::new(rx);
    let mut text = String::new();
    while let Some(Ok(data)) = rx.next().await {
        if data == "[DONE]" { break; }
        if data.starts_with("error:") {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        text.push_str(&data);
    }
    Ok(Json(GeminiChatResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: Some(GeminiRole::Model),
                parts: vec![GeminiPart::Text(text)],
            },
            finish_reason: Some(GeminiFinishReason::Stop),
            safety_ratings: None, citation_metadata: None,
            token_count: None, grounding_attributions: None, index: Some(0),
        }],
        prompt_feedback: None, usage_metadata: None, model_version: None,
    }).into_response())
}

async fn running(
    headers: HeaderMap,
    state: Arc<AgentState>,
    mut req: GeminiChatRequest,
    query: HashMap<String, String>,
    model: &str,
    tx: &Sender<Result<String, Infallible>>,
) -> Result<()> {
    let sid = session_id(&headers);

    let talk = req.contents.last()
        .and_then(|c| c.parts.iter().filter_map(|p| match p {
            GeminiPart::Text(s) => Some(s.clone()), _ => None
        }).next()).unwrap_or_default();

    let _ = tx.send(Ok(String::new())).await;
    let mut session = state.sessions.gain_session(&sid, &talk, "gemini", model)?;
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
            Path(format!("{}:generateContent", model)),
            Query(query.clone()),
            body,
        ).await.map_err(|e| anyhow!(e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(format!("HTTP {}", resp.status())));
        }

        let resp: GeminiChatResponse = deser_resp(resp).await?;
        let calls: Vec<(String, serde_json::Value)> = resp.candidates.iter()
            .flat_map(|c| &c.content.parts)
            .filter_map(|p| match p {
                GeminiPart::FunctionCall(fc) => {
                    Some((fc.name.clone(), fc.args.clone().unwrap_or_default()))
                }
                _ => None,
            }).collect();

        if calls.is_empty() {
            let content = resp.candidates.iter()
                .flat_map(|c| &c.content.parts)
                .filter_map(|p| match p {
                    GeminiPart::Text(s) => Some(s.clone()),
                    _ => None
                })
                .collect::<Vec<_>>().join("");
            let _ = state.sessions.create_message(&mut session, &content, "assistant")?;
            session.update();
            let _ = state.sessions.save_metadata(&session);
            let _ = tx.send(Ok(content)).await;
            let _ = tx.send(Ok("[DONE]".into())).await;
            return Ok(());
        }

        let _ = tx.send(Ok(format!("tool:{}", calls.len()))).await;
        msgs.push(GeminiContent {
            role: Some(GeminiRole::Model),
            parts: resp.candidates.iter().flat_map(|c| c.content.parts.clone()).collect(),
        });

        let mut tlparts = Vec::new();
        for (fn_name, fn_args) in &calls {
            let result = state.toolctx.execute(fn_name, fn_args);
            let c = if result.success { result.content.clone() } else { format!("ERROR: {}", result.content) };
            tlparts.push(GeminiPart::FunctionResponse(GeminiFunctionResponse {
                name: fn_name.clone(),
                response: serde_json::json!({ "result": &c }),
            }));
            let _ = state.sessions.create_message(&mut session, &c, "tool")?;
        }
        msgs.push(GeminiContent { role: Some(GeminiRole::User), parts: tlparts });
        session.update();
        let _ = state.sessions.save_metadata(&session);
    }
    Err(anyhow!("max rounds"))
}