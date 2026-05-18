import { useState, useCallback } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { TooltipProvider } from '@/components/tooltip';
import { Button } from '@/components/button';
import { Toaster } from 'sonner';
import { ArrowLeftIcon } from 'lucide-react';
import { useConversations } from '@crewride/core';
import ChatView from '@/views/chat';
import SettingsView from '@/views/settings';
import { Sidebar } from '@/layouts/sidebar';
import { Header, Theme } from '@/layouts/header';

function AppLayout() {
    const navigate = useNavigate();
    const {
        conversations,
        activeId,
        setActiveId,
        createConversation,
        deleteConversation,
    } = useConversations();

    const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
    const [model] = useState('gpt-4o');

    const handleSelectConversation = useCallback(
        (id: string) => setActiveId(id),
        [setActiveId],
    );

    const handleCreateConversation = useCallback(() => {
        createConversation('New Chat', model, 'openai');
    }, [model, createConversation]);

    const handleDeleteConversation = useCallback(
        (id: string) => deleteConversation(id),
        [deleteConversation],
    );

    const handleToggleSidebar = useCallback(
        () => setSidebarCollapsed((prev) => !prev),
        [],
    );

    const handleNavigateSettings = useCallback(() => {
        navigate('/settings');
    }, [navigate]);

    return (
        <div className="flex h-screen overflow-hidden bg-background">
            <Sidebar
                conversations={conversations}
                activeId={activeId}
                collapsed={sidebarCollapsed}
                onSelectConversation={handleSelectConversation}
                onCreateConversation={handleCreateConversation}
                onDeleteConversation={handleDeleteConversation}
                onToggleCollapse={handleToggleSidebar}
                onNavigateSettings={handleNavigateSettings}
            />
            <main className="flex flex-1 flex-col overflow-hidden">
                <Header>
                    <Theme />
                </Header>
                <ChatView />
            </main>
        </div>
    );
}

function SettingsLayout() {
    const navigate = useNavigate();
    return (
        <div className="flex h-screen flex-col bg-background">
            <Header title={
                <Button
                variant="ghost"
                size="sm"
                className="gap-2"
                onClick={() => navigate('/chat')}
                type="button"
            >
                <ArrowLeftIcon className="size-4" />
                Settings
            </Button>
        }>
                <Theme />
            </Header>
            <SettingsView />
        </div>
    )
}

function App() {
    return (
        <TooltipProvider delayDuration={300}>
            <BrowserRouter>
                <Routes>
                    <Route path="/chat" element={<AppLayout />} />
                    <Route path="/settings" element={<SettingsLayout />} />
                    <Route path="*" element={<Navigate to="/chat" replace />} />
                </Routes>
            </BrowserRouter>
            <Toaster
                position="top-center"
                richColors
                closeButton
                toastOptions={{
                    duration: 3000,
                }}
            />
        </TooltipProvider>
    );
}

export default App;
