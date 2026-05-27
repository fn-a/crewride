import { useState, useCallback, useRef } from 'react';
import { streamText } from 'ai';
import { useProvider } from './providers';
import type { ProviderKind, Message } from '../types';

// 构建消息对象
function toMessage(id: string, role: 'user' | 'assistant', content: string): Message {
    return {
        key: id,
        from: role,
        versions: [{ id, content }],
    };
}

// 通过 Rust 后端代理发送聊天请求，由 @ai-sdk/* 处理格式差异和 SSE 流解析
export function useChat() {
    const [messages, setMessages] = useState<Message[]>([]);
    const [status, setStatus] = useState<'submitted' | 'streaming' | 'ready' | 'error'>('ready');
    const abortRef = useRef<AbortController | null>(null);

    const sendMessage = useCallback(async (providerKind: ProviderKind, modelId: string, text: string) => {
        const model = useProvider(providerKind, modelId);
        const userMsgId = `user-${Date.now()}`;
        const assistantMsgId = `assistant-${Date.now()}`;

        // 添加用户消息 + 空的助手占位
        setMessages((prev) => [
            ...prev,
            toMessage(userMsgId, 'user', text),
            toMessage(assistantMsgId, 'assistant', ''),
        ]);

        setStatus('submitted');

        const controller = new AbortController();
        abortRef.current = controller;

        try {
            setStatus('streaming');

            const history = messages.map((m) => ({
                role: m.from as 'user' | 'assistant',
                content: m.versions[m.versions.length - 1].content,
            }));

            const result = streamText({
                model,
                messages: [...history, { role: 'user' as const, content: text }],
                abortSignal: controller.signal,
            });

            let fullContent = '';
            for await (const chunk of result.textStream) {
                fullContent += chunk;
                setMessages((prev) =>
                    prev.map((m) =>
                        m.key === assistantMsgId
                            ? { ...m, versions: [{ id: assistantMsgId, content: fullContent }] }
                            : m,
                    ),
                );
            }

            setStatus('ready');
        } catch (err: unknown) {
            if (err instanceof DOMException && err.name === 'AbortError') {
                setStatus('ready');
                return;
            }
            setStatus('error');
            const errMsg = err instanceof Error ? err.message : 'Unknown error';
            setMessages((prev) =>
                prev.map((m) =>
                    m.key === assistantMsgId
                        ? { ...m, versions: [{ id: assistantMsgId, content: `Error: ${errMsg}` }] }
                        : m,
                ),
            );
        }
    }, [messages]);

    const stop = useCallback(() => {
        abortRef.current?.abort();
    }, []);

    return { messages, status, sendMessage, stop };
}
