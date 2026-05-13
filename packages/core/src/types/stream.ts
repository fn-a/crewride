/** A parsed SSE event from the server */
export interface SSEEvent {
    id?: string;
    event?: string;
    data: string;
}

/** A chunk of streaming content */
export interface StreamChunk {
    content: string;
    done: boolean;
}

/** OpenAI streaming chunk structure */
export interface OpenAIStreamChunk {
    id: string;
    object: string;
    created: number;
    model: string;
    choices: {
        index: number;
        delta: {
            role?: string;
            content?: string;
        };
        finish_reason?: string | null;
    }[];
}

/** Anthropic streaming event types */
export type AnthropicStreamEventType =
    | 'message_start'
    | 'content_block_start'
    | 'content_block_delta'
    | 'content_block_stop'
    | 'message_delta'
    | 'message_stop'
    | 'error';

/** Anthropic streaming delta */
export interface AnthropicContentDelta {
    type: 'text_delta' | 'partial_json';
    text?: string;
    partial_json?: string;
}

/** Anthropic streaming chunk */
export interface AnthropicStreamChunk {
    type: AnthropicStreamEventType;
    index?: number;
    delta?: AnthropicContentDelta;
    message?: {
        id: string;
        model: string;
        usage?: { input_tokens: number; output_tokens: number };
    };
    content_block?: {
        type: string;
        text?: string;
    };
    usage?: {
        output_tokens?: number;
    };
}

/** Gemini streaming chunk — same structure as non-streaming response */
export interface GeminiStreamChunk {
    candidates: {
        content: {
            role?: string;
            parts: { text?: string }[];
        };
        finishReason?: string;
        index?: number;
    }[];
    usageMetadata?: {
        promptTokenCount: number;
        candidatesTokenCount?: number;
        totalTokenCount: number;
    };
}
