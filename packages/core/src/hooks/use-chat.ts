import { useState, useCallback, useRef } from 'react';
import type { Message, ChatSession, ChatStatus, ProviderType } from '../types';
import { CrewRideClient } from '../utils/api';
import { createMessage, generateId, deriveTitle } from '../utils/format';
import { parseSSEStream, extractStreamChunk } from '../utils/stream';
import { toOpenAIRequest, toAnthropicRequest, toGeminiRequest } from '../protocol';

export interface UseChatOptions {
    baseUrl?: string;
    apiKey?: string;
    model?: string;
    provider?: ProviderType;
    systemPrompt?: string;
}

export interface UseChatReturn {
    /** Current chat session state */
    session: ChatSession | null;
    /** Current status */
    status: ChatStatus;
    /** Error message if status is "error" */
    error: string | null;
    /** Create a new chat session */
    createSession: (model: string, provider: ProviderType) => void;
    /** Send a user message and get AI response */
    sendMessage: (content: string) => Promise<void>;
    /** Stop the current streaming response */
    stopStreaming: () => void;
    /** Clear the current session */
    clearSession: () => void;
}

/** Default configuration */
const DEFAULT_BASE_URL = '/v1';
const DEFAULT_MODEL = 'gpt-4o';
const DEFAULT_PROVIDER: ProviderType = 'openai';

/**
 * Core chat hook — manages a single chat session with streaming support.
 * This hook is provider-agnostic and delegates to protocol adapters.
 */
export function useChat(options: UseChatOptions = {}): UseChatReturn {
    const baseUrl = options.baseUrl ?? DEFAULT_BASE_URL;
    const apiKey = options.apiKey;

    const [session, setSession] = useState<ChatSession | null>(null);
    const [status, setStatus] = useState<ChatStatus>('idle');
    const [error, setError] = useState<string | null>(null);

    const abortRef = useRef<AbortController | null>(null);
    const clientRef = useRef(new CrewRideClient({ baseUrl, apiKey }));

    // Update client when config changes
    clientRef.current = new CrewRideClient({ baseUrl, apiKey });

    const createSession = useCallback(
        (model: string = DEFAULT_MODEL, provider: ProviderType = DEFAULT_PROVIDER) => {
            abortRef.current?.abort();

            const messages: Message[] = [];
            if (options.systemPrompt) {
                messages.push(createMessage('system', options.systemPrompt));
            }

            setSession({
                id: generateId(),
                title: 'New Chat',
                messages,
                status: 'idle',
                model,
                provider,
                createdAt: Date.now(),
                updatedAt: Date.now(),
            });
            setStatus('idle');
            setError(null);
        },
        [options.systemPrompt],
    );

    const stopStreaming = useCallback(() => {
        abortRef.current?.abort();
        abortRef.current = null;
        setStatus('idle');
    }, []);

    const clearSession = useCallback(() => {
        abortRef.current?.abort();
        setSession(null);
        setStatus('idle');
        setError(null);
    }, []);

    const sendMessage = useCallback(
        async (content: string) => {
            if (!session) return;

            const userMessage = createMessage('user', content);
            const updatedMessages = [...session.messages, userMessage];

            // Update session with user message
            setSession((prev) => {
                if (!prev) return prev;
                return {
                    ...prev,
                    messages: updatedMessages,
                    title:
                        prev.messages.filter((m) => m.role !== 'system').length === 0
                            ? deriveTitle(content)
                            : prev.title,
                    updatedAt: Date.now(),
                };
            });

            // Add placeholder assistant message
            const assistantMessage = createMessage('assistant', '');
            setStatus('streaming');
            setError(null);

            setSession((prev) => {
                if (!prev) return prev;
                return {
                    ...prev,
                    messages: [...updatedMessages, assistantMessage],
                    status: 'streaming',
                    updatedAt: Date.now(),
                };
            });

            try {
                const abortController = new AbortController();
                abortRef.current = abortController;

                const client = clientRef.current;
                let response: Response;

                switch (session.provider) {
                    case 'openai': {
                        const body = toOpenAIRequest(updatedMessages, session.model, {
                            stream: true,
                        });
                        response = await client.createOpenAIChat(body, {
                            signal: abortController.signal,
                        });
                        break;
                    }
                    case 'anthropic': {
                        const body = toAnthropicRequest(updatedMessages, session.model, {
                            stream: true,
                        });
                        response = await client.createAnthropicMessage(body, {
                            signal: abortController.signal,
                        });
                        break;
                    }
                    case 'gemini': {
                        const { body } = toGeminiRequest(updatedMessages);
                        response = await client.createGeminiStream(session.model, body, {
                            signal: abortController.signal,
                        });
                        break;
                    }
                }

                if (!response.ok) {
                    throw new Error(`API error: ${response.status} ${response.statusText}`);
                }

                // Stream the response
                let fullContent = '';
                const sseStream = parseSSEStream(response);

                for await (const event of sseStream) {
                    if (abortController.signal.aborted) break;

                    const chunk = extractStreamChunk(event, session.provider);
                    if (!chunk) continue;

                    if (chunk.done) break;

                    fullContent += chunk.content;
                    const currentContent = fullContent;

                    setSession((prev) => {
                        if (!prev) return prev;
                        const msgs = [...prev.messages];
                        const lastIdx = msgs.length - 1;
                        if (lastIdx >= 0 && msgs[lastIdx].role === 'assistant') {
                            msgs[lastIdx] = {
                                ...msgs[lastIdx],
                                content: currentContent,
                            } as Message & { isStreaming: true };
                        }
                        return { ...prev, messages: msgs, updatedAt: Date.now() };
                    });
                }

                // Finalize the assistant message
                setSession((prev) => {
                    if (!prev) return prev;
                    const msgs = [...prev.messages];
                    const lastIdx = msgs.length - 1;
                    if (lastIdx >= 0 && msgs[lastIdx].role === 'assistant') {
                        msgs[lastIdx] = {
                            ...msgs[lastIdx],
                            content: fullContent,
                        };
                    }
                    return {
                        ...prev,
                        messages: msgs,
                        status: 'idle',
                        updatedAt: Date.now(),
                    };
                });

                setStatus('idle');
            } catch (err) {
                if (err instanceof DOMException && err.name === 'AbortError') {
                    setStatus('idle');
                    return;
                }

                const errorMessage = err instanceof Error ? err.message : 'Unknown error';
                setError(errorMessage);
                setStatus('error');

                // Remove the empty assistant message on error
                setSession((prev) => {
                    if (!prev) return prev;
                    const msgs = prev.messages.filter(
                        (m) => !(m.role === 'assistant' && !m.content),
                    );
                    return {
                        ...prev,
                        messages: msgs,
                        status: 'error',
                        error: errorMessage,
                        updatedAt: Date.now(),
                    };
                });
            } finally {
                abortRef.current = null;
            }
        },
        [session],
    );

    return {
        session,
        status,
        error,
        createSession,
        sendMessage,
        stopStreaming,
        clearSession,
    };
}
