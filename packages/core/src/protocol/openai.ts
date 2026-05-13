import type { ProviderType, Message } from '../types';
import type { OpenAIChatRequest } from '../utils/api';

/**
 * Convert internal Messages to OpenAI request format.
 *
 * OpenAI endpoint: POST /v1/chat/completions
 * - system/user/assistant/tool roles
 * - content is string
 * - stream via `stream: true`
 */
export function toOpenAIRequest(
    messages: Message[],
    model: string,
    options?: {
        temperature?: number;
        maxTokens?: number;
        stream?: boolean;
    },
): OpenAIChatRequest {
    return {
        model,
        messages: messages.map((msg) => ({
            role: msg.role as 'system' | 'user' | 'assistant' | 'tool',
            content: msg.content,
        })),
        temperature: options?.temperature,
        max_tokens: options?.maxTokens,
        stream: options?.stream ?? true,
    };
}

/** Provider descriptor for the OpenAI protocol */
export const openaiProvider: { type: ProviderType; endpoint: string } = {
    type: 'openai',
    endpoint: '/v1/chat/completions',
};
