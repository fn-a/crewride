use std::path::Path;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model: String,
    pub name: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub replace: Option<ReplaceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceConfig {
    #[serde(default)]
    pub api_key: bool,
    pub model: Option<String>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8899
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

impl Config {
    /// 加载配置 (优先级: 配置文件 < 环境变量 < CLI 参数)
    pub fn load() -> Config {
        // 1. 尝试从配置文件加载
        if let Ok(config_path) = std::env::var("CREWRIDE_CONFIG_FILE") {
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
            eprintln!("⚠️  Warning: Failed to merge environment variables: {}", e);
        }
        config
    }

    /// 从 JSON 或 YAML 文件加载配置
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
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
                    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                        provider.api_key = Some(api_key);
                    }
                    if let Ok(api_url) = std::env::var("OPENAI_API_URL") {
                        provider.api_url = Some(api_url.parse().with_context(|| "Invalid OPENAI_API_URL format")?);
                    } else if provider.api_url.is_none() {
                        provider.api_url = Some(openai_api_url()?);
                    }
                }
                Provider::Anthropic => {
                    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
                        provider.api_key = Some(api_key);
                    }
                    if let Ok(api_url) = std::env::var("ANTHROPIC_API_URL") {
                        provider.api_url = Some(api_url.parse().with_context(|| "Invalid ANTHROPIC_API_URL format")?);
                    } else if provider.api_url.is_none() {
                        provider.api_url = Some(anthropic_api_url()?);
                    }
                }
                Provider::Gemini => {
                    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
                        provider.api_key = Some(api_key);
                    }
                    if let Ok(api_url) = std::env::var("GEMINI_API_URL") {
                        provider.api_url = Some(api_url.parse().with_context(|| "Invalid GEMINI_API_URL format")?);
                    } else if provider.api_url.is_none() {
                        provider.api_url = Some(gemini_api_url()?);
                    }
                }
            }
        }

        // 读取 host
        if let Ok(host) = std::env::var("CREWRIDE_PROXY_HOST") {
            self.host = host;
        }

        // 读取 port
        if let Ok(port) = std::env::var("CREWRIDE_PROXY_PORT") {
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
            providers: Vec::new(),
            models: Vec::new(),
        }
    }
}

pub struct ProxyState {
    pub client: reqwest::Client,
    pub config: Config,
}