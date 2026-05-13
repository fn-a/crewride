import type { ProviderType, Message } from '../types';
import type { GeminiRequest } from '../utils/api';

/**
 * Convert internal Messages to Gemini request format.
 *
 * Gemini endpoint: POST /v1beta/models/{model}:streamGenerateContent?alt=sse
 * - System prompt is in `systemInstruction`, NOT in contents
 * - Roles are "user" and "model" (not "assistant")
 * - Messages are wrapped in `parts: [{ text }]`
 * - Streaming requires `?alt=sse` query param
 */
export function toGeminiRequest(
    messages: Message[],
    options?: {
        temperature?: number;
        maxOutputTokens?: number;
    },
): { body: GeminiRequest } {
    // Extract system messages
    const systemMessage = messages.find((msg) => msg.role === 'system');

    // Convert messages to Gemini format
    const contents = messages
        .filter((msg) => msg.role !== 'system')
        .map((msg) => ({
            role: msg.role === 'assistant' ? ('model' as const) : ('user' as const),
            parts: [{ text: msg.content }],
        }));

    const body: GeminiRequest = {
        contents,
        ...(systemMessage && {
            systemInstruction: {
                parts: [{ text: systemMessage.content }],
            },
        }),
        generationConfig: {
            temperature: options?.temperature,
            maxOutputTokens: options?.maxOutputTokens,
        },
    };

    return { body };
}

/** Provider descriptor for the Gemini protocol */
export const geminiProvider: { type: ProviderType; endpoint: string } = {
    type: 'gemini',
    endpoint: '/v1beta/models',
};
