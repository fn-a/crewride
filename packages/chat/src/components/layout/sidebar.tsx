import { useCallback } from 'react';
import type { ProviderType, ChatSession } from '@crewride/core';
import mdiMenu from '~icons/mdi/menu';
import mdiPlus from '~icons/mdi/plus';
import mdiMessageOutline from '~icons/mdi/message-outline';
import mdiDeleteOutline from '~icons/mdi/delete-outline';
import { Button } from '@/components/ui/button';
import { Icon } from '@/components/ui/icon';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

interface SidebarProps {
    sessions: ChatSession[];
    activeSessionId: string | null;
    onSelectSession: (id: string) => void;
    onNewChat: (model: string, provider: ProviderType) => void;
    onDeleteSession: (id: string) => void;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
}

function Sidebar({
    sessions,
    activeSessionId,
    onSelectSession,
    onNewChat,
    onDeleteSession,
    collapsed = false,
    onToggleCollapse,
}: SidebarProps) {
    const handleNewChat = useCallback(() => {
        onNewChat('gpt-4o', 'openai');
    }, [onNewChat]);

    if (collapsed) {
        return (
            <div className="flex w-12 flex-col items-center border-r bg-background py-2">
                <Button variant="ghost" size="icon" onClick={onToggleCollapse} className="mb-2">
                    <Icon raw={mdiMenu} className="h-5 w-5" />
                </Button>
                <Button variant="ghost" size="icon" onClick={handleNewChat} className="mb-2">
                    <Icon raw={mdiPlus} className="h-5 w-5" />
                </Button>
            </div>
        );
    }

    return (
        <div className="flex w-64 flex-col border-r bg-background">
            {/* Header */}
            <div className="flex items-center justify-between p-3">
                <h2 className="text-sm font-semibold">Chats</h2>
                <div className="flex gap-1">
                    <Button variant="ghost" size="icon" onClick={handleNewChat}>
                        <Icon raw={mdiPlus} className="h-4 w-4" />
                    </Button>
                    <Button variant="ghost" size="icon" onClick={onToggleCollapse}>
                        <Icon raw={mdiMenu} className="h-4 w-4" />
                    </Button>
                </div>
            </div>

            <Separator />

            {/* Session list */}
            <div className="flex-1 overflow-y-auto p-2">
                {sessions.length === 0 ? (
                    <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                        No conversations yet
                    </p>
                ) : (
                    sessions.map((session) => (
                        <div
                            key={session.id}
                            className={cn(
                                'group flex cursor-pointer items-center rounded-md px-2 py-1.5 text-sm hover:bg-accent',
                                activeSessionId === session.id &&
                                    'bg-accent text-accent-foreground',
                            )}
                            onClick={() => onSelectSession(session.id)}
                        >
                            <Icon
                                raw={mdiMessageOutline}
                                className="mr-2 h-4 w-4 shrink-0 text-muted-foreground"
                            />
                            <span className="flex-1 truncate">{session.title}</span>
                            <Button
                                variant="ghost"
                                size="icon"
                                className="h-6 w-6 opacity-0 group-hover:opacity-100"
                                onClick={(e) => {
                                    e.stopPropagation();
                                    onDeleteSession(session.id);
                                }}
                            >
                                <Icon raw={mdiDeleteOutline} className="h-3 w-3" />
                            </Button>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}

export { Sidebar };
