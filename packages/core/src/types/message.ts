/** Message role in a chat conversation */
export type MessageRole = 'system' | 'user' | 'assistant';

/** A single chat message */
export interface Message {
    id: string;
    role: MessageRole;
    content: string;
    timestamp: number;
    metadata?: MessageMetadata;
}

/** Additional metadata attached to a message */
export interface MessageMetadata {
    model?: string;
    provider?: string;
    tokenCount?: number;
    latency?: number;
}

/** A message that is currently being streamed */
export interface StreamingMessage extends Message {
    isStreaming: true;
}
