use std::env;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use url::Url;
use aidapter::Provider;

// ============ 配置结构 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_public")]
    pub public: String,
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub models: Vec<ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub key: String,
    pub name: String,
    pub r#type: Provider,
    pub api_key: Option<String>,
    pub api_url: Option<Url>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub retry: Option<RetryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model: String,
    pub name: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default, skip_serializing)]
    pub byokey: bool,
    #[serde(skip_serializing)]
    pub remodel: Option<String>,
}

// ============ Token 用量 ============

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub requests: u64,
    pub tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageStats {
    pub requests: AtomicU64,
    pub tokens: AtomicU64,
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            requests: AtomicU64::new(0),
            tokens: AtomicU64::new(0),
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
        }
    }
}

impl UsageStats {
    pub fn record(&self, usage: &TokenUsage) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.input_tokens.fetch_add(usage.input_tokens, Ordering::Relaxed);
        self.output_tokens.fetch_add(usage.output_tokens, Ordering::Relaxed);
        self.tokens.fetch_add(usage.tokens, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TokenUsage {
        TokenUsage {
            requests: self.requests.load(Ordering::Relaxed),
            input_tokens: self.input_tokens.load(Ordering::Relaxed),
            output_tokens: self.output_tokens.load(Ordering::Relaxed),
            tokens: self.tokens.load(Ordering::Relaxed),
        }
    }
}

// ============ 重试配置 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_retry_maxnum")]
    pub retry_maxnum: u32,
    #[serde(default = "default_retry_status")]
    pub retry_status: Vec<u16>,
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,
    #[serde(default = "default_most_delay_ms")]
    pub most_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            retry_maxnum: default_retry_maxnum(),
            retry_status: default_retry_status(),
            base_delay_ms: default_base_delay_ms(),
            most_delay_ms: default_most_delay_ms(),
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8899
}

fn default_public() -> String {
    "/public".to_string()
}

fn default_true() -> bool {
    true
}

fn openai_api_url() -> Result<Url> {
    "https://api.openai.com/"
        .parse::<Url>()
        .context("Failed to parse OpenAI API URL")
}

fn anthropic_api_url() -> Result<Url> {
    "https://api.anthropic.com/"
        .parse::<Url>()
        .context("Failed to parse Anthropic API URL")
}

fn gemini_api_url() -> Result<Url> {
    "https://generativelanguage.googleapis.com/"
        .parse::<Url>()
        .context("Failed to parse Gemini API URL")
}

fn default_retry_maxnum() -> u32 {
    3 
}

fn default_base_delay_ms() -> u64 {
    500 
}

fn default_most_delay_ms() -> u64 {
    10000 
}

fn default_retry_status() -> Vec<u16> {
    vec![429, 502, 503, 504] 
}

impl Config {
    /// 加载配置 (优先级: 配置文件 < 环境变量 < CLI 参数)
    pub fn load() -> Config {
        // 1. 尝试从配置文件加载
        if let Ok(config_path) = env::var("CREWRIDE_CONFIG_FILE") {
            println!("📄 Loading config from: {}", config_path);
            match Config::from_file(&config_path) {
                Ok(config) => {
                    println!("✅ Config loaded successfully");
                    return config;
                }
                Err(e) => {
                    eprintln!("❌ Failed to load config: {}", e);
                    eprintln!("   Falling back to environment variables");
                }
            }
        }

        // 2. 尝试自动搜索配置文件
        if let Ok(current_dir) = std::env::current_dir() {
            println!("🔍 Searching for config files in current directory");
            match Config::from_dir(&current_dir) {
                Ok(config) => {
                    println!("✅ Config loaded automatically");
                    return config;
                }
                Err(e) => {
                    println!("📝 No config file found: {}", e);
                }
            }
        }

        // 3. 使用默认配置 + 环境变量
        let mut config = Config::default();
        if let Err(e) = config.merge_env() {
            eprintln!("⚠️ Warning: Failed to merge environment variables: {}", e);
        }
        config
    }

    /// 从 JSON 或 YAML 文件加载配置
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let mut config: Config = match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .as_deref()
        {
            Some("json") => {
                println!("📄 Loading JSON config from: {}", path.display());
                serde_json::from_str(&contents).with_context(|| {
                    format!("Failed to parse JSON config from: {}", path.display())
                })?
            }
            Some("yaml") | Some("yml") => {
                println!("📄 Loading YAML config from: {}", path.display());
                serde_yaml::from_str(&contents).with_context(|| {
                    format!("Failed to parse YAML config from: {}", path.display())
                })?
            }
            Some(ext) => return Err(anyhow!("Unsupported file extension: .{}", ext)),
            None => return Err(anyhow!("File must have an extension (.json or .yaml)")),
        };

        // 合并环境变量
        config.merge_env().with_context(|| {
            format!("Failed to merge environment variables for config: {}", path.display())
        })?;

        Ok(config)
    }

    /// 自动搜索并加载配置文件（支持 JSON 和 YAML）
    /// 搜索优先级: config.json, config.yaml, config.yml
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        println!("🔍 Searching for config files in: {}", dir.display());

        // 按优先级搜索配置文件
        let candidates = ["config.json", "config.yaml", "config.yml"];

        for filename in &candidates {
            let config_path = dir.join(filename);
            if config_path.exists() {
                println!("✅ Found config file: {}", config_path.display());
                return Self::from_file(config_path);
            }
        }

        Err(anyhow!("No config file found in directory: {}", dir.display()))
    }

    /// 合并环境变量 (环境变量优先级更高)
    pub fn merge_env(&mut self) -> Result<()> {
        // 更新提供商配置中的API密钥和URL
        for provider in &mut self.providers {
            match provider.r#type {
                Provider::OpenAI => {
                    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
                        provider.api_key = Some(api_key);
                    }
                    if let Ok(api_url) = env::var("OPENAI_API_URL") {
                        provider.api_url = Some(api_url.parse().with_context(|| "Invalid OPENAI_API_URL format")?);
                    } else if provider.api_url.is_none() {
                        provider.api_url = Some(openai_api_url()?);
                    }
                }
                Provider::Anthropic => {
                    if let Ok(api_key) = env::var("ANTHROPIC_API_KEY") {
                        provider.api_key = Some(api_key);
                    }
                    if let Ok(api_url) = env::var("ANTHROPIC_API_URL") {
                        provider.api_url = Some(api_url.parse().with_context(|| "Invalid ANTHROPIC_API_URL format")?);
                    } else if provider.api_url.is_none() {
                        provider.api_url = Some(anthropic_api_url()?);
                    }
                }
                Provider::Gemini => {
                    if let Ok(api_key) = env::var("GEMINI_API_KEY") {
                        provider.api_key = Some(api_key);
                    }
                    if let Ok(api_url) = env::var("GEMINI_API_URL") {
                        provider.api_url = Some(api_url.parse().with_context(|| "Invalid GEMINI_API_URL format")?);
                    } else if provider.api_url.is_none() {
                        provider.api_url = Some(gemini_api_url()?);
                    }
                }
            }
        }

        // 读取 host
        if let Ok(host) = env::var("CREWRIDE_PROXY_HOST") {
            self.host = host;
        }

        // 读取 port
        if let Ok(port) = env::var("CREWRIDE_PROXY_PORT") {
            if let Ok(port_num) = port.parse() {
                self.port = port_num;
            }
        }

        Ok(())
    }

    /// 验证配置
    pub fn validate(&self) {
        for provider in &self.providers {
            if provider.enabled && provider.api_key.is_none() {
                eprintln!("⚠️  Warning: {} Api Key not set", provider.key);
            }
        }
    }

    /// 根据模型名查找模型配置
    pub fn find_model(&self, model: &str) -> Option<&ModelConfig> {
        self.models.iter().find(|m| m.model == model)
    }

    /// 根据提供商key查找提供商配置
    pub fn find_provider(&self, key: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.key == key && p.enabled)
    }

    /// 根据提供商类型查找出一个可用供商配置
    pub fn give_provider(&self, r#type: Provider) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.r#type == r#type && p.enabled)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: default_host(),
            port: default_port(),
            public: default_public(),
            providers: Vec::new(),
            models: Vec::new(),
        }
    }
}

impl ProviderConfig {
    pub fn provider(&self) -> String {
        match self.r#type {
            Provider::OpenAI => "openai".to_string(),
            Provider::Anthropic => "anthropic".to_string(),
            Provider::Gemini => "gemini".to_string(),
        }
    }
}

pub struct AdaptState {
    pub client: reqwest::Client,
    pub config: Config,
    pub stats: UsageStats,
}
