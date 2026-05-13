import { useState, useCallback } from 'react';
import type { ProviderType, ChatSession } from '@crewride/core';
import { useChat } from '@crewride/core';
import { TooltipProvider } from '@/components/ui/tooltip';
import { ChatPanel } from '@/components/chat';
import { Sidebar } from '@/components/layout';

function App() {
    const { session, status, error, createSession, sendMessage, stopStreaming, clearSession } =
        useChat();

    const [sessions, setSessions] = useState<ChatSession[]>([]);
    const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

    const handleNewChat = useCallback(
        (model: string, provider: ProviderType) => {
            createSession(model, provider);
        },
        [createSession],
    );

    const handleSelectSession = useCallback(
        (id: string) => {
            const found = sessions.find((s) => s.id === id);
            if (found) {
                // Multi-session management would require extending useChat
            }
        },
        [sessions],
    );

    const handleDeleteSession = useCallback(
        (id: string) => {
            setSessions((prev) => prev.filter((s) => s.id !== id));
            if (session?.id === id) {
                clearSession();
            }
        },
        [session, clearSession],
    );

    const handleModelChange = useCallback(
        (model: string, provider: ProviderType) => {
            createSession(model, provider);
        },
        [createSession],
    );

    return (
        <TooltipProvider>
            <div className="flex h-screen bg-background text-foreground">
                <Sidebar
                    sessions={sessions}
                    activeSessionId={session?.id ?? null}
                    onSelectSession={handleSelectSession}
                    onNewChat={handleNewChat}
                    onDeleteSession={handleDeleteSession}
                    collapsed={sidebarCollapsed}
                    onToggleCollapse={() => setSidebarCollapsed((v) => !v)}
                />
                <main className="flex flex-1 flex-col">
                    {error && (
                        <div className="bg-destructive/10 px-4 py-2 text-sm text-destructive">
                            {error}
                        </div>
                    )}
                    <ChatPanel
                        session={session}
                        status={status}
                        onSend={sendMessage}
                        onStop={stopStreaming}
                        onModelChange={handleModelChange}
                    />
                </main>
            </div>
        </TooltipProvider>
    );
}

export default App;
