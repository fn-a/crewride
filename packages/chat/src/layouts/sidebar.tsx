import * as React from 'react';
import { cn } from '@/utils/style';
import { Button } from '@/components/button';
import { ScrollArea } from '@/components/scroll-area';
import { Separator } from '@/components/separator';
import {
    PlusIcon,
    MessageSquareIcon,
    Trash2Icon,
    SettingsIcon,
    PanelLeftCloseIcon,
    PanelLeftIcon,
} from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/tooltip';
import type { Conversation } from '@crewride/core';

interface SidebarProps {
    conversations: Conversation[];
    activeId: string | null;
    collapsed: boolean;
    onSelectConversation: (id: string) => void;
    onCreateConversation: () => void;
    onDeleteConversation: (id: string) => void;
    onToggleCollapse: () => void;
    onNavigateSettings: () => void;
}

export function Sidebar({
    conversations,
    activeId,
    collapsed,
    onSelectConversation,
    onCreateConversation,
    onDeleteConversation,
    onToggleCollapse,
    onNavigateSettings,
}: SidebarProps) {
    return (
        <aside
            className={cn(
                'flex h-full flex-col border-r bg-muted/30 transition-all duration-300',
                collapsed ? 'w-13' : 'w-65',
            )}
        >
            {/* Header */}
            <div
                className={cn(
                    'flex items-center border-b p-2',
                    collapsed ? 'flex-col gap-2' : 'justify-between',
                )}
            >
                {!collapsed && (
                    <span className="text-sm font-semibold px-2">Chats</span>
                )}
                <div className={cn('flex items-center gap-1', collapsed && 'flex-col')}>
                    {!collapsed && (
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <Button
                                    variant="ghost"
                                    size="icon-sm"
                                    onClick={onCreateConversation}
                                    type="button"
                                >
                                    <PlusIcon className="size-4" />
                                </Button>
                            </TooltipTrigger>
                            <TooltipContent side="right">New Chat</TooltipContent>
                        </Tooltip>
                    )}
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button
                                variant="ghost"
                                size="icon-sm"
                                onClick={onToggleCollapse}
                                type="button"
                            >
                                {collapsed ? (
                                    <PanelLeftIcon className="size-4" />
                                ) : (
                                    <PanelLeftCloseIcon className="size-4" />
                                )}
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent side="right">
                            {collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
                        </TooltipContent>
                    </Tooltip>
                </div>
            </div>

            {/* Conversation list */}
            <ScrollArea className="flex-1">
                <div className="p-2 space-y-1">
                    {conversations.map((conv) => (
                        <ConversationItem
                            key={conv.id}
                            conversation={conv}
                            isActive={conv.id === activeId}
                            collapsed={collapsed}
                            onSelect={() => onSelectConversation(conv.id)}
                            onDelete={() => onDeleteConversation(conv.id)}
                        />
                    ))}
                    {conversations.length === 0 && !collapsed && (
                        <p className="px-3 py-8 text-center text-xs text-muted-foreground">
                            No conversations yet. Click + to start
                        </p>
                    )}
                </div>
            </ScrollArea>

            {/* Footer */}
            <Separator />
            <div className={cn('p-2', collapsed && 'flex justify-center')}>
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            variant="ghost"
                            size={collapsed ? 'icon-sm' : 'sm'}
                            className={cn('w-full', collapsed ? '' : 'justify-start gap-2')}
                            onClick={onNavigateSettings}
                            type="button"
                        >
                            <SettingsIcon className="size-4" />
                            {!collapsed && <span className="text-xs">Settings</span>}
                        </Button>
                    </TooltipTrigger>
                    {collapsed && <TooltipContent side="right">Settings</TooltipContent>}
                </Tooltip>
            </div>
        </aside>
    );
}

function ConversationItem({
    conversation,
    isActive,
    collapsed,
    onSelect,
    onDelete,
}: {
    conversation: Conversation;
    isActive: boolean;
    collapsed: boolean;
    onSelect: () => void;
    onDelete: () => void;
}) {
    const [showDelete, setShowDelete] = React.useState(false);

    if (collapsed) {
        return (
            <Tooltip>
                <TooltipTrigger asChild>
                    <button
                        type="button"
                        onClick={onSelect}
                        className={cn(
                            'flex w-full items-center justify-center rounded-md p-2 text-xs transition-colors',
                            isActive
                                ? 'bg-accent text-accent-foreground'
                                : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground',
                        )}
                    >
                        <MessageSquareIcon className="size-4" />
                    </button>
                </TooltipTrigger>
                <TooltipContent side="right">{conversation.title}</TooltipContent>
            </Tooltip>
        );
    }

    return (
        <div
            className={cn(
                'group flex items-center rounded-md pr-1 transition-colors',
                isActive
                    ? 'bg-accent text-accent-foreground'
                    : 'text-foreground hover:bg-accent/50',
            )}
            onMouseEnter={() => setShowDelete(true)}
            onMouseLeave={() => setShowDelete(false)}
        >
            <button
                type="button"
                onClick={onSelect}
                className="flex min-w-0 flex-1 items-center gap-2 overflow-hidden py-2 pl-2 text-left text-sm"
            >
                <MessageSquareIcon className="size-4 shrink-0 text-muted-foreground" />
                <span className="truncate">{conversation.title}</span>
            </button>
            <Button
                variant="ghost"
                size="icon-sm"
                className={cn(
                    'size-6 shrink-0 transition-all',
                    showDelete
                        ? 'opacity-100'
                        : 'opacity-0 pointer-events-none',
                )}
                onClick={(e) => {
                    e.stopPropagation();
                    onDelete();
                }}
                type="button"
            >
                <Trash2Icon className="size-3" />
            </Button>
        </div>
    );
}
