import type { ProviderType } from '../types';

/** Configuration for the CrewRide API client */
export interface CrewRideClientConfig {
    baseUrl: string;
    apiKey?: string;
}

/** Generic request options for any provider */
export interface RequestOptions {
    signal?: AbortSignal;
}

/** OpenAI chat completion request body */
export interface OpenAIChatRequest {
    model: string;
    messages: {
        role: 'system' | 'user' | 'assistant' | 'tool';
        content: string;
    }[];
    temperature?: number;
    max_tokens?: number;
    stream?: boolean;
}

/** Anthropic messages request body */
export interface AnthropicMessageRequest {
    model: string;
    messages: {
        role: 'user' | 'assistant';
        content: string;
    }[];
    max_tokens: number;
    system?: string;
    temperature?: number;
    stream?: boolean;
}

/** Gemini generate content request body */
export interface GeminiRequest {
    contents: {
        role?: 'user' | 'model';
        parts: { text: string }[];
    }[];
    systemInstruction?: {
        parts: { text: string }[];
    };
    generationConfig?: {
        temperature?: number;
        maxOutputTokens?: number;
    };
}

/**
 * CrewRide API client — communicates with the Rust backend.
 *
 * The backend exposes three endpoints matching the provider formats:
 * - POST /v1/chat/completions      (OpenAI)
 * - POST /v1/messages              (Anthropic)
 * - POST /v1beta/models/{model}:streamGenerateContent (Gemini)
 */
export class CrewRideClient {
    private baseUrl: string;
    private apiKey?: string;

    constructor(config: CrewRideClientConfig) {
        this.baseUrl = config.baseUrl.replace(/\/$/, '');
        this.apiKey = config.apiKey;
    }

    /** Send a chat request using the OpenAI format */
    async createOpenAIChat(body: OpenAIChatRequest, options?: RequestOptions): Promise<Response> {
        return this.fetch('/v1/chat/completions', body, 'openai', options);
    }

    /** Send a chat request using the Anthropic format */
    async createAnthropicMessage(
        body: AnthropicMessageRequest,
        options?: RequestOptions,
    ): Promise<Response> {
        return this.fetch('/v1/messages', body, 'anthropic', options);
    }

    /** Send a chat request using the Gemini format (streaming) */
    async createGeminiStream(
        model: string,
        body: GeminiRequest,
        options?: RequestOptions,
    ): Promise<Response> {
        const keyParam = this.apiKey ? `&key=${this.apiKey}` : '';
        return this.fetch(
            `/v1beta/models/${model}:streamGenerateContent?alt=sse${keyParam}`,
            body,
            'gemini',
            options,
        );
    }

    /** Send a chat request using the Gemini format (non-streaming) */
    async createGeminiContent(
        model: string,
        body: GeminiRequest,
        options?: RequestOptions,
    ): Promise<Response> {
        const keyParam = this.apiKey ? `?key=${this.apiKey}` : '';
        return this.fetch(
            `/v1beta/models/${model}:generateContent${keyParam}`,
            body,
            'gemini',
            options,
        );
    }

    private async fetch(
        path: string,
        body: unknown,
        provider: ProviderType,
        options?: RequestOptions,
    ): Promise<Response> {
        const headers = this.buildHeaders(provider);
        return globalThis.fetch(`${this.baseUrl}${path}`, {
            method: 'POST',
            headers,
            body: JSON.stringify(body),
            signal: options?.signal,
        });
    }

    private buildHeaders(provider: ProviderType): Record<string, string> {
        const headers: Record<string, string> = {
            'Content-Type': 'application/json',
        };

        if (this.apiKey) {
            switch (provider) {
                case 'openai':
                    headers['Authorization'] = `Bearer ${this.apiKey}`;
                    break;
                case 'anthropic':
                    headers['x-api-key'] = this.apiKey;
                    headers['anthropic-version'] = '2023-06-01';
                    break;
                // Gemini uses query params, not headers
            }
        }

        return headers;
    }
}
