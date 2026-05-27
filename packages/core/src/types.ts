import type { ToolUIPart } from 'ai';

export type ProviderKind = 'openai' | 'anthropic' | 'gemini';

export interface ProviderConfig {
    apiKey?: string;
    baseURL?: string;
}

export interface MessageVersion {
    id: string;
    content: string;
}

export interface MessageTooling {
    name: string;
    description: string;
    status: ToolUIPart['state'];
    parameters: Record<string, unknown>;
    result: string | undefined;
    error: string | undefined;
}

export interface MessageSource {
    href: string;
    title: string;
}

export interface Message {
    key: string;
    from: 'user' | 'assistant';
    sources?: MessageSource[];
    versions: MessageVersion[];
    reasoning?: {
        content: string;
        duration: number;
    };
    tools?: MessageTooling[];
}


export interface Session {
    id: string;
    title: string;
    modelId: string;
    providerKind: ProviderKind;
    messages: Message[];
    createdAt: number;
    updatedAt: number;
}

// 用量统计数据
export interface TokenUsage {
    requests: number;
    input_tokens: number;
    output_tokens: number;
    tokens: number;
}