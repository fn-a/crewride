import { useCallback, useMemo, useState } from 'react';
import type { Conversation, ChatMessage, ProviderKind } from './types';

function generateId(): string {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) {
        return crypto.randomUUID();
    }
    return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export function useConversations() {
    const [conversations, setConversations] = useState<Conversation[]>([]);
    const [activeId, setActiveId] = useState<string | null>(null);

    const activeConversation = useMemo(
        () => conversations.find((c) => c.id === activeId) ?? null,
        [conversations, activeId],
    );

    const createConversation = useCallback(
        (title: string, modelId: string, providerKind: ProviderKind) => {
            const now = Date.now();
            const conv: Conversation = {
                id: generateId(),
                title,
                modelId,
                providerKind,
                messages: [],
                createdAt: now,
                updatedAt: now,
            };
            setConversations((prev) => [conv, ...prev]);
            setActiveId(conv.id);
            return conv;
        },
        [],
    );

    const deleteConversation = useCallback((id: string) => {
        setConversations((prev) => prev.filter((c) => c.id !== id));
        setActiveId((prev) => (prev === id ? null : prev));
    }, []);

    const updateConversation = useCallback(
        (id: string, updates: Partial<Conversation>) => {
            setConversations((prev) =>
                prev.map((c) =>
                    c.id === id ? { ...c, ...updates, updatedAt: Date.now() } : c,
                ),
            );
        },
        [],
    );

    const addMessage = useCallback((conversationId: string, message: ChatMessage) => {
        setConversations((prev) =>
            prev.map((c) =>
                c.id === conversationId
                    ? { ...c, messages: [...c.messages, message], updatedAt: Date.now() }
                    : c,
            ),
        );
    }, []);

    const updateMessage = useCallback(
        (conversationId: string, messageKey: string, updater: (msg: ChatMessage) => ChatMessage) => {
            setConversations((prev) =>
                prev.map((c) =>
                    c.id === conversationId
                        ? {
                              ...c,
                              messages: c.messages.map((m) =>
                                  m.key === messageKey ? updater(m) : m,
                              ),
                              updatedAt: Date.now(),
                          }
                        : c,
                ),
            );
        },
        [],
    );

    return {
        conversations,
        activeId,
        activeConversation,
        setActiveId,
        createConversation,
        deleteConversation,
        updateConversation,
        addMessage,
        updateMessage,
    };
}
