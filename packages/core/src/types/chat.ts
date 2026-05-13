import type { Message, MessageMetadata } from './message';
import type { ProviderType } from './provider';

/** Current status of a chat session */
export type ChatStatus = 'idle' | 'loading' | 'streaming' | 'error';

/** A chat session containing messages and configuration */
export interface ChatSession {
    id: string;
    title: string;
    messages: Message[];
    status: ChatStatus;
    model: string;
    provider: ProviderType;
    createdAt: number;
    updatedAt: number;
    error?: string;
}

/** Options for creating a new chat session */
export interface CreateChatSessionOptions {
    model: string;
    provider: ProviderType;
    systemPrompt?: string;
}

/** Result of sending a message */
export interface SendMessageResult {
    message: Message;
    metadata?: MessageMetadata;
}
