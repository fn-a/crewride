use std::fs;
use std::process::Command;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use aidapter::{
    openai::prefix::{OpenAITool, OpenAIFunction},
    anthropic::prefix::AnthropicTool,
    gemini::prefix::{GeminiFunctionDeclaration, GeminiSchema},
};

#[derive(Debug, Clone)]
pub struct ToolDefine {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value, // JSON Schema
}

// 工具执行上下文
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub workspace: PathBuf,
    pub tools: Vec<ToolDefine>,
}

impl ToolContext {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self { workspace: workspace.into(), tools: Self::registry() }
    }

    pub fn filte(&mut self, filter: &Option<Vec<String>>) {
        let reg = Self::registry();
        self.tools = match filter {
            None => vec![],
            Some(names) if names.is_empty() => reg,
            Some(names) => reg.into_iter().filter(|t| names.contains(&t.name.to_string())).collect(),
        }
    }

    /// 路径安全校验：防止目录穿越攻击
    pub fn fence(&self, path: &str) -> Result<PathBuf, String> {
        let p = Path::new(path);
        let resolved = if p.is_absolute() {
            self.workspace.join(p.strip_prefix("/").unwrap_or(p))
        } else {
            self.workspace.join(p)
        };
        let rl_canonical = fs::canonicalize(&resolved).unwrap_or(resolved.clone());
        let wk_canonical = fs::canonicalize(&self.workspace).unwrap_or(self.workspace.clone());
        if !rl_canonical.starts_with(&wk_canonical) {
            Err(format!("access denied: path '{}' is outside workspace", path))
        } else {
            Ok(resolved)
        }
    }

    // 注册工具
    pub fn registry() -> Vec<ToolDefine> {
        vec![
            read_file_def(),
            write_file_def(),
            remove_file_def(),
            create_file_def(),
            read_dir_def(),
            make_dir_def(),
            grep_txt_def(),
            exec_cmd_def(),
        ]
    }

    // 工具执行
    pub fn execute(&self, name: &str, args: &Value) -> ToolResult {
        let result = match name {
            "read_file" => read_file_exe(self, args),
            "write_file" => write_file_exe(self, args),
            "remove_file" => remove_file_exe(self, args),
            "create_file" => create_file_exe(self, args),
            "read_dir" => read_dir_exe(self, args),
            "make_dir" => make_dir_exe(self, args),
            "grep_txt" => grep_txt_exe(self, args),
            "exec_cmd" => exec_cmd_exe(self, args),
            _ => Err(format!("unknown tool: {}", name)),
        };
        match result {
            Ok(content) => ToolResult { success: true, content },
            Err(e) => ToolResult { success: false, content: e },
        }
    }
}

// 执行结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
}

// 读取文件 read_file
fn read_file_def() -> ToolDefine {
    ToolDefine {
        name: "read_file",
        description: "读取文件内容，返回文本",
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "相对于工作目录的文件路径" }
            },
            "required": ["path"]
        }),
    }
}

fn read_file_exe(ctx: &ToolContext, args: &serde_json::Value) -> Result<String, String> {
    let path = args["path"].as_str().ok_or("missing path")?;
    let full_path = ctx.fence(path)?;
    if !full_path.is_file() {
        Err(format!("not a file: {}", path))
    } else {
        fs::read_to_string(&full_path).map_err(|e| format!("read error: {e}"))
    }
}

// 写入文件 write_file
fn write_file_def() -> ToolDefine {
    ToolDefine {
        name: "write_file",
        description: "写入内容到文件（覆盖模式）",
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "相对于工作目录的文件路径" },
                "content": { "type": "string", "description": "要写入的内容" }
            },
            "required": ["path", "content"]
        }),
    }
}

fn write_file_exe(ctx: &ToolContext, args: &serde_json::Value) -> Result<String, String> {
    let path = args["path"].as_str().ok_or("missing path")?;
    let content = args["content"].as_str().ok_or("missing content")?;
    let full_path = ctx.fence(path)?;
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
    }
    fs::write(&full_path, content).map_err(|e| format!("write error: {e}"))?;
    Ok(format!("written {} bytes to {}", content.len(), path))
}

// 删除文件 remove_file
fn remove_file_def() -> ToolDefine {
    ToolDefine {
        name: "remove_file",
        description: "删除指定文件",
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "相对于工作目录的文件路径" }
            },
            "required": ["path"]
        }),
    }
}

fn remove_file_exe(ctx: &ToolContext, args: &serde_json::Value) -> Result<String, String> {
    let path = args["path"].as_str().ok_or("missing path")?;
    let full_path = ctx.fence(path)?;
    if !full_path.is_file() {
        Err(format!("not a file: {}", path))
    } else {
        fs::remove_file(&full_path).map_err(|e| format!("remove error: {e}"))?;
        Ok(format!("deleted {}", path))
    }
}

// 创建文件 create_file
fn create_file_def() -> ToolDefine {
    ToolDefine {
        name: "create_file",
        description: "创建新文件（含父目录），如已存在则不做任何操作",
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "相对于工作目录的文件路径" },
                "content": { "type": "string", "description": "可选的初始内容" }
            },
            "required": ["path"]
        }),
    }
}

fn create_file_exe(ctx: &ToolContext, args: &serde_json::Value) -> Result<String, String> {
    let path = args["path"].as_str().ok_or("missing path")?;
    let full_path = ctx.fence(path)?;
    if full_path.exists() {
        Err(format!("file already exists: {}", path))
    } else {
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
        }
        let content = args["content"].as_str().unwrap_or("");
        fs::write(&full_path, content).map_err(|e| format!("create error: {e}"))?;
        Ok(format!("created {}", path))
    }
}

// 读取目录 read_dir
fn read_dir_def() -> ToolDefine {
    ToolDefine {
        name: "read_dir",
        description: "列出目录内容，返回文件和子目录名",
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "相对于工作目录的路径，空字符串表示根目录" }
            },
            "required": []
        }),
    }
}

fn read_dir_exe(ctx: &ToolContext, args: &Value) -> Result<String, String> {
    let rel = args["path"].as_str().unwrap_or("");
    let full_path = if rel.is_empty() {
        ctx.workspace.clone()
    } else {
        ctx.fence(rel)?
    };
    if !full_path.is_dir() {
        Err(format!("not a directory: {}", rel))
    } else {
        let mut entries = vec![];
        for entry in fs::read_dir(&full_path).map_err(|e| format!("read_dir error: {e}"))? {
            let entry = entry.map_err(|e| format!("entry error: {e}"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let prefix = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { "[D]" } else { "[F]" };
            entries.push(format!("{} {}", prefix, name));
        }
        if entries.is_empty() {
            Ok("(empty)".into())
        } else {
            Ok(entries.join("\n"))
        }
    }
}

// 创建目录 make_dir
fn make_dir_def() -> ToolDefine {
    ToolDefine {
        name: "make_dir",
        description: "创建目录（含父目录）",
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "相对于工作目录的目录路径" }
            },
            "required": ["path"]
        }),
    }
}

fn make_dir_exe(ctx: &ToolContext, args: &Value) -> Result<String, String> {
    let path = args["path"].as_str().ok_or("missing path")?;
    let full_path = ctx.fence(path)?;
    if full_path.exists() {
        Err(format!("directory already exists: {}", path))
    } else {
        fs::create_dir_all(&full_path).map_err(|e| format!("mkdir error: {e}"))?;
        Ok(format!("created directory {}", path))
    }
}

// 搜索文本 grep_txt
fn grep_txt_def() -> ToolDefine {
    ToolDefine {
        name: "grep_txt",
        description: "在指定路径（文件或目录）中递归搜索文本",
        parameters: json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "搜索的文本（子串匹配）" },
                "path": { "type": "string", "description": "文件或目录路径，空字符串表示整个工作目录" }
            },
            "required": ["pattern"]
        }),
    }
}

fn grep_txt_exe(ctx: &ToolContext, args: &Value) -> Result<String, String> {
    let pattern = args["pattern"].as_str().ok_or("missing pattern")?;
    let rel = args["path"].as_str().unwrap_or("");
    let search_root = if rel.is_empty() {
        ctx.workspace.clone()
    } else {
        ctx.fence(rel)?
    };

    let mut results = vec![];
    search_files(&search_root, pattern, &mut results)?;

    if results.is_empty() {
        Ok("no matches".into())
    } else {
        Ok(results.join("\n"))
    }
}

fn search_files(root: &Path, pattern: &str, results: &mut Vec<String>) -> Result<(), String> {
    if root.is_file() {
        let content = fs::read_to_string(root).map_err(|e| format!("read error: {e}"))?;
        for (i, line) in content.lines().enumerate() {
            if line.contains(pattern) {
                results.push(format!("{}:{}: {}", root.display(), i + 1, line));
            }
        }
    } else if root.is_dir() {
        search_dirs(root, pattern, results)?;
    }
    Ok(())
}

fn search_dirs(root: &Path, pattern: &str, results: &mut Vec<String>) -> Result<(), String> {
    if root.is_dir() {
        for entry in fs::read_dir(root).map_err(|e| format!("read_dir error: {e}"))? {
            let entry = entry.map_err(|e| format!("entry error: {e}"))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            // 跳过隐藏文件和常见忽略目录
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if path.is_dir() || path.is_file() {
                search_files(&path, pattern, results)?;
            }
        }
    }
    Ok(())
}

// 执行系统命令 exec_cmd
fn exec_cmd_def() -> ToolDefine {
    ToolDefine {
        name: "exec_cmd",
        description: "在工作目录中执行系统命令，返回 stdout + stderr",
        parameters: json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "要执行的 shell 命令" }
            },
            "required": ["command"]
        }),
    }
}

fn exec_cmd_exe(ctx: &ToolContext, args: &Value) -> Result<String, String> {
    let command = args["command"].as_str().ok_or("missing command")?;
    let output = Command::new(if cfg!(windows) { "cmd" } else { "sh" })
        .arg(if cfg!(windows) { "/C" } else { "-c" })
        .arg(command)
        .current_dir(&ctx.workspace)
        .output()
        .map_err(|e| format!("exec error: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() { result.push('\n'); }
        result.push_str("stderr:\n");
        result.push_str(&stderr);
    }
    if result.is_empty() {
        result = format!("exit code: {}", output.status.code().unwrap_or(-1));
    }
    Ok(result)
}

impl From<&ToolDefine> for GeminiFunctionDeclaration {
    fn from(td: &ToolDefine) -> Self {
        Self {
            name: td.name.into(),
            description: Some(td.description.into()),
            parameters: Some(GeminiSchema::from(&td.parameters)),
        }
    }
}

impl From<&ToolDefine> for AnthropicTool {
    fn from(td: &ToolDefine) -> Self {
        Self {
            name: td.name.into(), 
            description: Some(td.description.into()),
            input_schema: td.parameters.clone(),
        }
    }
}

impl From<&ToolDefine> for OpenAITool {
    fn from(td: &ToolDefine) -> Self {
        Self {
            r#type: "function".into(),
            function: OpenAIFunction {
                name: td.name.into(),
                description: Some(td.description.into()),
                parameters: td.parameters.clone()
            },
        }
    }
}