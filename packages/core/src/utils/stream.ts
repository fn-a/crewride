import type { SSEEvent, StreamChunk, ProviderType } from '../types';

/**
 * Parse an SSE stream from a Response object.
 * Handles both unnamed events (`data: ...`) and named events (`event: ...\ndata: ...`).
 */
export async function* parseSSEStream(response: Response): AsyncGenerator<SSEEvent> {
    const reader = response.body?.getReader();
    if (!reader) {
        throw new Error('Response body is not readable');
    }

    const decoder = new TextDecoder();
    let buffer = '';

    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            buffer += decoder.decode(value, { stream: true });
            const lines = buffer.split('\n');
            // Keep the last (potentially incomplete) line in the buffer
            buffer = lines.pop() ?? '';

            let currentEvent: string | undefined;
            let currentData = '';

            for (const line of lines) {
                if (line.startsWith('event:')) {
                    currentEvent = line.slice(6).trim();
                } else if (line.startsWith('data:')) {
                    currentData += line.slice(5).trim();
                } else if (line === '') {
                    // Empty line signals end of event
                    if (currentData) {
                        yield {
                            event: currentEvent,
                            data: currentData,
                        };
                    }
                    currentEvent = undefined;
                    currentData = '';
                }
            }
        }

        // Flush remaining buffer
        if (buffer.startsWith('data:')) {
            yield {
                data: buffer.slice(5).trim(),
            };
        }
    } finally {
        reader.releaseLock();
    }
}

/**
 * Extract text content from an SSE event based on the provider type.
 * Each provider has a different streaming format.
 */
export function extractStreamChunk(event: SSEEvent, provider: ProviderType): StreamChunk | null {
    if (event.data === '[DONE]') {
        return { content: '', done: true };
    }

    try {
        const json = JSON.parse(event.data);

        switch (provider) {
            case 'openai':
                return extractOpenAIChunk(json);
            case 'anthropic':
                return extractAnthropicChunk(event.event, json);
            case 'gemini':
                return extractGeminiChunk(json);
            default:
                return null;
        }
    } catch {
        return null;
    }
}

function extractOpenAIChunk(json: Record<string, unknown>): StreamChunk | null {
    const choices = json.choices as
        | Array<{
              delta?: { content?: string };
              finish_reason?: string | null;
          }>
        | undefined;

    if (!choices?.length) return null;

    const choice = choices[0];
    if (choice.finish_reason) {
        return { content: '', done: true };
    }

    const content = choice.delta?.content ?? '';
    return { content, done: false };
}

function extractAnthropicChunk(
    eventType: string | undefined,
    json: Record<string, unknown>,
): StreamChunk | null {
    switch (eventType) {
        case 'content_block_delta': {
            const delta = json.delta as { type: string; text?: string } | undefined;
            const content = delta?.type === 'text_delta' ? (delta.text ?? '') : '';
            return { content, done: false };
        }
        case 'message_stop':
            return { content: '', done: true };
        case 'error': {
            const error = json.error as { message?: string } | undefined;
            throw new Error(error?.message ?? 'Anthropic stream error');
        }
        default:
            return null;
    }
}

function extractGeminiChunk(json: Record<string, unknown>): StreamChunk | null {
    const candidates = json.candidates as
        | Array<{
              content?: { parts?: Array<{ text?: string }> };
              finishReason?: string;
          }>
        | undefined;

    if (!candidates?.length) return null;

    const candidate = candidates[0];
    if (candidate.finishReason) {
        return { content: '', done: true };
    }

    const text = candidate.content?.parts?.[0]?.text ?? '';
    return { content: text, done: false };
}
