import { useState, useCallback, useRef } from 'react';
import { streamText } from 'ai';
import { useProvider } from './providers';
import type { ProviderKind, Message, MessageTooling, ChatState } from '../types';
import { SESSION } from '../config';

// 通过 Rust 后端代理发送聊天请求，由 @ai-sdk/* 处理格式差异和 SSE 流解析
export function useChat(sessionId?: string | null) {
    const [messages, setMessages] = useState<Message[]>([]);
    const [status, setStatus] = useState<ChatState>('ready');
    const abortRef = useRef<AbortController | null>(null);

    const headers = sessionId ? { [SESSION]: sessionId } : undefined;

    const sendMessage = useCallback(async (providerKind: ProviderKind, modelId: string, text: string) => {
        const model = useProvider(providerKind, modelId);
        const userMsgId = `user-${Date.now()}`;
        const assistantMsgId = `assistant-${Date.now()}`;

        // 添加用户消息 + 空的助手占位
        setMessages((prev) => [
            ...prev,
            {
                key: userMsgId,
                from: 'user',
                versions: [{ id: userMsgId, content: text }],
            },
            {
                key: assistantMsgId,
                from: 'assistant',
                versions: [{ id: assistantMsgId, content: '' }],
            },
        ]);

        setStatus('submitted');

        const controller = new AbortController();
        abortRef.current = controller;

        try {
            setStatus('streaming');

            const result = streamText({
                model,
                messages: [{ role: 'user' as const, content: text }],
                abortSignal: controller.signal,
                headers,
                onStepFinish(step) {
                    // 捕获思考过程
                    if (step.reasoning) {
                        const reasoningText = typeof step.reasoning === 'string'
                            ? step.reasoning
                            : (step.reasoning as { text?: string }).text ?? '';
                        if (reasoningText) {
                            setMessages((prev) =>
                                prev.map((m) =>
                                    m.key === assistantMsgId
                                        ? { ...m, reasoning: { content: reasoningText, duration: 0 } }
                                        : m,
                                ),
                            );
                        }
                    }

                    // 捕获工具调用
                    if (step.toolCalls?.length || step.toolResults?.length) {
                        const tcMap = new Map<string, MessageTooling>();
                        // 先处理 toolCalls（创建条目）
                        for (const tc of (step.toolCalls || [])) {
                            tcMap.set(tc.toolCallId, {
                                name: tc.toolName,
                                description: '',
                                status: 'input-available',
                                parameters: tc.input,
                                result: undefined,
                                error: undefined,
                            });
                        }
                        // 再处理 toolResults（填充结果）
                        for (const tr of (step.toolResults || [])) {
                            const existing = tcMap.get(tr.toolCallId);
                            if (existing) {
                                existing.result = tr.output;
                                existing.status = 'output-available';
                            }
                        }
                        const toolList = Array.from(tcMap.values());
                        if (toolList.length) {
                            setMessages((prev) =>
                                prev.map((m) =>
                                    m.key === assistantMsgId
                                        ? { ...m, tools: toolList }
                                        : m,
                                ),
                            );
                        }
                    }
                },
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
