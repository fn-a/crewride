import { useCallback, useMemo, useState } from 'react';
import type { Session, Message, ProviderKind } from '../types';

function generateId(): string {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) {
        return crypto.randomUUID();
    }
    return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export function useSessions() {
    const [sessions, setSessions] = useState<Session[]>([]);
    const [activeId, setActiveId] = useState<string | null>(null);

    const activeSession = useMemo(
        () => sessions.find((s) => s.id === activeId) ?? null,
        [sessions, activeId],
    );

    const createSession = useCallback(
        (title: string, modelId: string, providerKind: ProviderKind) => {
            const now = Date.now();
            const session: Session = {
                id: generateId(),
                title,
                modelId,
                providerKind,
                messages: [],
                createdAt: now,
                updatedAt: now,
            };
            setSessions((prev) => [session, ...prev]);
            setActiveId(session.id);
            return session;
        },
        [],
    );

    const deleteSession = useCallback((id: string) => {
        setSessions((prev) => prev.filter((s) => s.id !== id));
        setActiveId((prev) => (prev === id ? null : prev));
    }, []);

    const updateSession = useCallback(
        (id: string, updates: Partial<Session>) => {
            setSessions((prev) =>
                prev.map((s) =>
                    s.id === id ? { ...s, ...updates, updatedAt: Date.now() } : s,
                ),
            );
        },
        [],
    );

    const addMessage = useCallback((sessionId: string, message: Message) => {
        setSessions((prev) =>
            prev.map((s) =>
                s.id === sessionId
                    ? { ...s, messages: [...s.messages, message], updatedAt: Date.now() }
                    : s,
            ),
        );
    }, []);

    const updateMessage = useCallback(
        (sessionId: string, messageKey: string, updater: (msg: Message) => Message) => {
            setSessions((prev) =>
                prev.map((s) =>
                    s.id === sessionId
                        ? {
                              ...s,
                              messages: s.messages.map((m) =>
                                  m.key === messageKey ? updater(m) : m,
                              ),
                              updatedAt: Date.now(),
                          }
                        : s,
                ),
            );
        },
        [],
    );

    return {
        sessions,
        activeId,
        activeSession,
        setActiveId,
        createSession,
        deleteSession,
        updateSession,
        addMessage,
        updateMessage,
    };
}
