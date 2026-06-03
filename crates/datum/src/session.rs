use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

use aidapter::{
    openai::prefix::{OpenAIMessage, OpenAIMessageContent},
    anthropic::prefix::{AnthropicMessage, AnthropicContent, AnthropicRole},
    gemini::prefix::{GeminiContent, GeminiPart, GeminiRole},
};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub model: String,
    pub provider: String,
    pub messages: Vec<Message>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub model: String,
    pub provider: String,
    #[serde(default, skip_deserializing)]
    pub messages: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnippet {
    pub id: Option<String>,
    pub title: String,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub key: String,
    pub from: String, // "user" | "assistant"
    pub versions: Vec<MessageVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ModelReasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolcalls: Option<Vec<ModelToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageVersion {
    pub id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReasoning {
    pub content: String,
    pub duration: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelToolCall {
    pub name: String,
    pub status: ModelToolStatus,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelToolStatus {
    Pending,
    Done,
    Error,
}

pub struct SessionStore {
    sessions_dir: PathBuf,
    metadata_dir: PathBuf,
}

impl SessionStore {
    pub fn new(cfgdir: impl Into<PathBuf>) -> Self {
        let base = cfgdir.into();
        let sessions_dir = base.join("sessions");
        let metadata_dir = base.join("metadatas");
        let _ = fs::create_dir_all(&sessions_dir);
        let _ = fs::create_dir_all(&metadata_dir);
        Self { sessions_dir, metadata_dir }
    }

    fn metadata_path(&self, id: &str) -> PathBuf {
        self.metadata_dir.join(format!("{}.json", id))
    }

    fn messages_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.jsonl", id))
    }

    /// 加载会话 metadata
    pub fn load_metadata(&self, id: &str) -> Option<Session> {
        let data = fs::read_to_string(self.metadata_path(id)).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// 保存会话 metadata
    pub fn save_metadata(&self, session: &Session) -> Result<()> {
        let meta = serde_json::json!({
            "id": session.id,
            "title": session.title,
            "model": session.model,
            "provider": session.provider,
            "created_at": session.created_at,
            "updated_at": session.updated_at,
        });
        let path = self.metadata_path(&session.id);
        let json = serde_json::to_string_pretty(&meta).map_err(|e| anyhow!("serialize meta: {e}"))?;
        fs::write(&path, json).map_err(|e| anyhow!("write meta: {e}"))
    }

    /// 加载会话（metadata + messages）
    pub fn load_session(&self, id: &str) -> Option<Session> {
        let mut session = self.load_metadata(id)?;
        session.messages = self.load_messages(id)?;
        Some(session)
    }

    /// 获得会话（创建或加载）
    pub fn gain_session(&self, snippet: SessionSnippet) -> Result<Session> {
        let session = snippet.id.as_ref().map(|id| self.load_session(id)).flatten();
        if let Some(session) = session {
            Ok(session)
        } else {
            let session = Session::from(snippet);
            self.save_session(&session)?;
            Ok(session)
        }
    }

    /// 加载会话 messages
    pub fn load_messages(&self, id: &str) -> Option<Vec<Message>> {
        let path = self.messages_path(id);
        let file = fs::File::open(&path).ok()?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();
        for line in reader.lines() {
            if let Ok(line) = line {
                if let Ok(msg) = serde_json::from_str::<Message>(&line) {
                    messages.push(msg);
                }
            }
        }
        Some(messages)
    }

    /// 保存会话（metadata + 全量写入 messages JSONL）
    pub fn save_session(&self, session: &Session) -> Result<()> {
        self.save_metadata(session)?;
        // 全量重写 messages
        let path = self.messages_path(&session.id);
        let mut file = fs::File::create(&path).map_err(|e| anyhow!("create jsonl: {e}"))?;
        for msg in &session.messages {
            let line = serde_json::to_string(msg).map_err(|e| anyhow!("serialize msg: {e}"))?;
            writeln!(file, "{}", line).map_err(|e| anyhow!("write jsonl: {e}"))?;
        }
        Ok(())
    }

    /// 列出所有会话摘要
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut summaries = Vec::new();
        for entry in fs::read_dir(&self.metadata_dir).map_err(|e| anyhow!("read_dir: {e}"))? {
            let entry = entry.map_err(|e| anyhow!("entry: {e}"))?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(data) = fs::read_to_string(&path) {
                    let count = self.load_messages(
                        path.file_stem().unwrap_or_default().to_str().unwrap_or("")
                    ).map(|m| m.len()).unwrap_or(0);
                    let mut summary = serde_json::from_str::<SessionSummary>(&data).unwrap_or(SessionSummary { 
                        id: "".to_string(),
                        title: "".to_string(),
                        model: "".to_string(),
                        provider: "".to_string(),
                        messages: 0,
                        created_at: 0,
                        updated_at: 0,
                    });
                    summary.messages = count;
                    summaries.push(summary);
                }
            }
        }
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    /// 创建消息
    pub fn create_message(&self, session: &mut Session, msg: &str, from: &str) -> Result<()> {
        let msg = Message {
            key: format!("u-{}", Utc::now().timestamp_millis()), from: from.into(),
            versions: vec![MessageVersion { id: Uuid::new_v4().to_string(), content: msg.into() }],
            reasoning: None, toolcalls: None,
        };
        let _ = self.append_message(&session.id, &msg)?;
        session.messages.push(msg);
        Ok(())
    }

    /// 追加消息到 JSONL 文件
    pub fn append_message(&self, id: &str, msg: &Message) -> Result<()> {
        let path = self.messages_path(id);
        let mut file = fs::OpenOptions::new()
            .create(true).append(true)
            .open(&path)
            .map_err(|e| anyhow!("open jsonl: {e}"))?;
        let line = serde_json::to_string(msg).map_err(|e| anyhow!("serialize msg: {e}"))?;
        writeln!(file, "{}", line).map_err(|e| anyhow!("write jsonl: {e}"))?;
        Ok(())
    }

    /// 删除会话
    pub fn delete_session(&self, id: &str) -> Result<()> {
        let _ = fs::remove_file(self.messages_path(id));
        let _ = fs::remove_file(self.metadata_path(id));
        Ok(())
    }
}

impl Session {
    pub fn id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn update(&mut self) {
        self.updated_at = Utc::now().timestamp_millis() as u64;
    }
}

impl From<SessionSnippet> for Session {
    fn from(snippet: SessionSnippet) -> Self {
        Session {
            id: snippet.id.unwrap_or(Session::id()),
            title: snippet.title.chars().take(50).collect(),
            model: snippet.model,
            provider: snippet.provider,
            messages: Vec::new(),
            created_at: Utc::now().timestamp_millis() as u64,
            updated_at: Utc::now().timestamp_millis() as u64,
        }
    }
}

impl From<&Message> for GeminiContent {
    fn from(m: &Message) -> Self {
        let text = m.versions.last().map(|v| v.content.clone()).unwrap_or_default();
        let role = match m.from.as_str() { "assistant" | "model" => GeminiRole::Model, _ => GeminiRole::User };
        GeminiContent { role: Some(role), parts: vec![GeminiPart::Text(text)] }
    }
}

impl From<&Message> for AnthropicMessage {
    fn from(m: &Message) -> Self {
        let text = m.versions.last().map(|v| v.content.clone()).unwrap_or_default();
        let role = match m.from.as_str() { "assistant" => AnthropicRole::Assistant, _ => AnthropicRole::User };
        AnthropicMessage { role, content: AnthropicContent::Text(text) }
    }
}

impl From<&Message> for OpenAIMessage {
    fn from(m: &Message) -> Self {
        let text = m.versions.last().map(|v| v.content.clone()).unwrap_or_default();
        match m.from.as_str() {
            "assistant" => OpenAIMessage::Assistant {
                audio: None, content: Some(OpenAIMessageContent::String(text)),
                function_call: None, name: None, refusal: None, tool_calls: None,
            },
            "tool" => OpenAIMessage::Tool { content: OpenAIMessageContent::String(text), tool_call_id: String::new() },
            _ => OpenAIMessage::User { content: OpenAIMessageContent::String(text), name: None },
        }
    }
}
