import { useCallback, useEffect, useMemo, useState } from 'react';
import type { Session, ProviderKind } from '../types';
import { BASE_URL } from '../config';

const SESSIONS_API = '/api/sessions';

export function useSessions() {
    const [sessions, setSessions] = useState<Session[]>([]);
    const [activeId, setActiveId] = useState<string | null>(null);

    const refresh = useCallback(async () => {
        const listApi = new URL(SESSIONS_API, BASE_URL);
        const response = await fetch(listApi);
        if (!response.ok) {
            throw new Error(`Failed to fetch sessions: ${response.status}`);
        }
        const list: Session[] = await response.json();
        setSessions(list);
    }, []);

    // 首次加载从后端获取会话列表
    useEffect(() => {
        refresh();
    }, []);

    const activeSession = useMemo(
        () => sessions.find((s) => s.id === activeId) ?? null,
        [sessions, activeId],
    );

    const createSession = useCallback(
        async (title: string, model: string, provider: ProviderKind) => {
            const crateApi = new URL(SESSIONS_API, BASE_URL);
            const response = await fetch(crateApi, {
                method: 'POST',
                body: JSON.stringify({ title, model, provider }),
            });
            if (!response.ok) {
                throw new Error(`Failed to create session: ${response.status}`);
            }
            const session: Session = await response.json();
            setSessions((prev) => [session, ...prev]);
            setActiveId(session.id);
            return session;
        },
        [],
    );

    const deleteSession = useCallback(async (id: string) => {
        // 调用后端删除会话
        const deleteApi = new URL(`${SESSIONS_API}/${id}`, BASE_URL);
        const response = await fetch(deleteApi, {
            method: 'DELETE',
        });
        if (!response.ok) {
            throw new Error(`Failed to delete session: ${response.status}`);
        }
        // 刷新当前会话
        setSessions((prev) => prev.filter((s) => s.id !== id));
        setActiveId((prev) => (prev === id ? null : prev));
    }, []);

    return {
        sessions,
        activeId,
        setActiveId,
        activeSession,
        createSession,
        deleteSession,
        refresh,
    };
}
