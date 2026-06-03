import type { ToolUIPart } from 'ai';

export const ProviderKinds = ['openai', 'anthropic', 'gemini'] as const;

export type ProviderKind = typeof ProviderKinds[number];

export const ChatStates = ['submitted', 'streaming', 'ready', 'error'] as const;

export type ChatState = typeof ChatStates[number];

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
    model: string;
    provider: ProviderKind;
    messages: number | Message[];
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

export interface ModelInfo {
    model: string;
    name: string;
    provider: ProviderKind;
}