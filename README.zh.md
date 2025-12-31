# 实现AI供应商接口格式之间的互相转发

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
      "replace": {
        "api_key": true,
        "model": "gpt-5"
      }
    },
    {
      "model": "claude-3-sonnet",
      "name": "Anthropic Claude-3-Sonnet",
      "provider": "anthropic-official",
      "replace": {
        "api_key": false
      }
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
    replace: 
      - api_key: true
        model: "gpt-5"
  - model: "claude-3-sonnet"
    name: "Anthropic Claude-3-Sonnet"
    provider: "anthropic-official"
    replace: 
      - api_key: false
  - model: "gemini-pro"
    name: "Google Gemini Pro"
    provider: "gemini-official"
  - model: "qwen-2.5"
    name: "Local Qwen-2.5"
    provider: null
```

## 参考
https://deepwiki.com/erans/lunaroute
https://github.com/erans/lunaroute