import type { ToolUIPart } from 'ai';

export type ProviderKind = 'openai' | 'anthropic' | 'google';

export interface ProviderInfo {
    id: string;
    kind: ProviderKind;
    name: string;
    baseURL?: string;
    apiKey?: string;
}

export interface ProviderConfig {
    apiKey?: string;
    baseURL?: string;
}

export interface ModelInfo {
    id: string;
    name: string;
    chef: string;
    chefSlug: string;
    providers: string[];
}

export interface MessageVersion {
    id: string;
    content: string;
}

export interface MessageToolCall {
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

export interface ChatMessage {
    key: string;
    from: 'user' | 'assistant';
    versions: MessageVersion[];
    sources?: MessageSource[];
    reasoning?: {
        content: string;
        duration: number;
    };
    tools?: MessageToolCall[];
}


export interface Conversation {
    id: string;
    title: string;
    modelId: string;
    providerKind: ProviderKind;
    messages: ChatMessage[];
    createdAt: number;
    updatedAt: number;
}

export interface AgentConfig {
    modelId: string;
    providerKind: ProviderKind;
    systemPrompt?: string;
    temperature?: number;
    maxTokens?: number;
    tools?: Record<string, unknown>;
}

export interface ChatRequestOptions {
    conversationId?: string;
    modelId: string;
    providerKind: ProviderKind;
    messages: { role: 'user' | 'assistant'; content: string }[];
    systemPrompt?: string;
    temperature?: number;
    maxTokens?: number;
}

export interface MessageType {
    key: string;
    from: 'user' | 'assistant';
    sources?: { href: string; title: string }[];
    versions: {
        id: string;
        content: string;
    }[];
    reasoning?: {
        content: string;
        duration: number;
    };
    tools?: {
        name: string;
        description: string;
        status: ToolUIPart['state'];
        parameters: Record<string, unknown>;
        result: string | undefined;
        error: string | undefined;
    }[];
}