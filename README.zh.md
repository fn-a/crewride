# 实现AI供应商接口格式之间的互相转发的应用

## 测试 Anthropic → OpenAI
curl -X POST http://localhost:8899/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
  
## 测试 OpenAI → Anthropic
curl -X POST http://localhost:8899/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
  
## 测试流式响应
curl -N -X POST http://localhost:8899/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "max_tokens": 100,
    "stream": true,
    "messages": [{"role": "user", "content": "Count to 5"}]
  }'

## 📁 示例配置文件

### JSON 格式 (config.json)

```json
{
  "host": "127.0.0.1",
  "port": 8899,
  "providers": [
    {
      "key": "openai-official",
      "name": "OpenAI 官方",
      "type": "openai",
      "api_key": "${OPENAI_API_KEY}",
      "api_url": "http://127.0.0.1:11434/",
      "enabled": true
    },
    {
      "key": "anthropic-official",
      "name": "Anthropic 官方",
      "type": "anthropic",
      "api_key": "${ANTHROPIC_API_KEY}",
      "api_url": "https://api.anthropic.com/",
      "enabled": true
    },
    {
      "key": "gemini-official",
      "name": "Google Gemini 官方",
      "type": "gemini",
      "api_key": "${GEMINI_API_KEY}",
      "api_url": "https://generativelanguage.googleapis.com/",
      "enabled": true
    }
  ],
  "models": [
    {
      "model": "gpt-4",
      "name": "OpenAI GPT-4",
      "provider": "openai-official",
      "byokey": false,
      "remodel": "gpt-5"
    },
    {
      "model": "claude-3-sonnet",
      "name": "Anthropic Claude-3-Sonnet",
      "provider": "anthropic-official",
      "byokey": true
    },
    {
      "model": "gemini-pro",
      "name": "Google Gemini Pro",
      "provider": "gemini-official"
    },
    {
      "model": "qwen2.5:0.5b",
      "name": "Local Qwen-2.5",
      "provider": "openai-official"
    }
  ]
}
```

### YAML 格式 (config.yaml)

```yaml
host: "127.0.0.1"
port: 8899
providers:
  - key: "openai-official"
    name: "OpenAI 官方"
    type: "openai"
    api_key: "${OPENAI_API_KEY}"
    api_url: "https://api.openai.com/"
    enabled: true
  - key: "anthropic-official"
    name: "Anthropic 官方"
    type: "anthropic"
    api_key: "${ANTHROPIC_API_KEY}"
    api_url: "https://api.anthropic.com/"
    enabled: true
  - key: "gemini-official"
    name: "Google Gemini 官方"
    type: "gemini"
    api_key: "${GEMINI_API_KEY}"
    api_url: "https://generativelanguage.googleapis.com/"
    enabled: true
models:
  - model: "gpt-4"
    name: "OpenAI GPT-4"
    provider: "openai-official"
    byokey: false
    remodel: "gpt-5"
  - model: "claude-3-sonnet"
    name: "Anthropic Claude-3-Sonnet"
    provider: "anthropic-official"
    byokey: true
  - model: "gemini-pro"
    name: "Google Gemini Pro"
    provider: "gemini-official"
  - model: "qwen-2.5"
    name: "Local Qwen-2.5"
    provider: null
```

## 规划
增加AI Chat页面，开发Agent功能
页面演进功能，通过AI按照接口来生成页面，可加入到系统中，以便访问页面快速使用功能
根据对话时间线，自由调整对话流，可回溯从某次对话中切出新的对话流


## 开发时链接本地库

### 1️⃣ 在本地包目录执行

```bash
cd ../some/lib
pnpm link --global
```

### 2️⃣ 在项目目录执行

```bash
cd ../../crewride
pnpm link --global @some/lib
```

### 🔁 恢复远程版本
```bash
pnpm unlink @some/lib
pnpm install  # 重新拉取远程版本
```

## 参考
https://deepwiki.com/erans/lunaroute
https://github.com/erans/lunaroute
