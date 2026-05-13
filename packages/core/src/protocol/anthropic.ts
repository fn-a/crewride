import type { ProviderType, Message } from '../types';
import type { AnthropicMessageRequest } from '../utils/api';

/**
 * Convert internal Messages to Anthropic request format.
 *
 * Anthropic endpoint: POST /v1/messages
 * - System prompt is a top-level field, NOT in messages
 * - Only user/assistant roles in messages
 * - max_tokens is required
 * - stream via `stream: true`
 */
export function toAnthropicRequest(
    messages: Message[],
    model: string,
    options?: {
        maxTokens?: number;
        temperature?: number;
        stream?: boolean;
    },
): AnthropicMessageRequest {
    // Extract system messages from the message list
    const systemMessage = messages.find((msg) => msg.role === 'system');
    const chatMessages = messages
        .filter((msg) => msg.role !== 'system')
        .map((msg) => ({
            role: msg.role as 'user' | 'assistant',
            content: msg.content,
        }));

    return {
        model,
        messages: chatMessages,
        max_tokens: options?.maxTokens ?? 4096,
        system: systemMessage?.content,
        temperature: options?.temperature,
        stream: options?.stream ?? true,
    };
}

/** Provider descriptor for the Anthropic protocol */
export const anthropicProvider: { type: ProviderType; endpoint: string } = {
    type: 'anthropic',
    endpoint: '/v1/messages',
};
