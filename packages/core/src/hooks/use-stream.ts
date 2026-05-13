import { useState, useCallback, useRef } from 'react';
import type { ProviderType, StreamChunk } from '../types';
import { parseSSEStream, extractStreamChunk } from '../utils/stream';

export interface UseStreamOptions {
    /** Provider type — determines how SSE events are parsed */
    provider: ProviderType;
    /** Called for each text chunk received */
    onChunk?: (chunk: StreamChunk) => void;
    /** Called when the stream completes */
    onDone?: (fullContent: string) => void;
    /** Called on error */
    onError?: (error: Error) => void;
}

export interface UseStreamReturn {
    /** Whether a stream is currently active */
    isStreaming: boolean;
    /** Accumulated content from the stream */
    content: string;
    /** Start streaming from a Response object */
    startStream: (response: Response) => Promise<void>;
    /** Abort the current stream */
    abort: () => void;
    /** Reset the stream state */
    reset: () => void;
}

/**
 * Low-level SSE streaming hook.
 * Useful when you need direct control over the streaming response,
 * e.g., for custom UI that doesn't fit the useChat pattern.
 */
export function useStream(options: UseStreamOptions): UseStreamReturn {
    const [isStreaming, setIsStreaming] = useState(false);
    const [content, setContent] = useState('');
    const abortRef = useRef<AbortController | null>(null);
    const contentRef = useRef('');

    const abort = useCallback(() => {
        abortRef.current?.abort();
        abortRef.current = null;
        setIsStreaming(false);
    }, []);

    const reset = useCallback(() => {
        abort();
        setContent('');
        contentRef.current = '';
    }, [abort]);

    const startStream = useCallback(
        async (response: Response) => {
            if (!response.ok) {
                const err = new Error(`HTTP ${response.status}: ${response.statusText}`);
                options.onError?.(err);
                return;
            }

            setIsStreaming(true);
            contentRef.current = '';
            setContent('');

            const abortController = new AbortController();
            abortRef.current = abortController;

            try {
                const sseStream = parseSSEStream(response);

                for await (const event of sseStream) {
                    if (abortController.signal.aborted) break;

                    const chunk = extractStreamChunk(event, options.provider);
                    if (!chunk) continue;

                    if (chunk.done) {
                        options.onDone?.(contentRef.current);
                        break;
                    }

                    contentRef.current += chunk.content;
                    setContent(contentRef.current);
                    options.onChunk?.(chunk);
                }
            } catch (err) {
                if (err instanceof DOMException && err.name === 'AbortError') {
                    return;
                }
                const error = err instanceof Error ? err : new Error(String(err));
                options.onError?.(error);
            } finally {
                setIsStreaming(false);
                abortRef.current = null;
            }
        },
        [options],
    );

    return {
        isStreaming,
        content,
        startStream,
        abort,
        reset,
    };
}
