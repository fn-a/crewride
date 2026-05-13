import type { ChatStatus } from '@crewride/core';
import { cn } from '@/lib/utils';

interface ChatHeaderProps {
    status: ChatStatus;
}

const statusConfig: Record<ChatStatus, { label: string; dotClass: string }> = {
    idle: { label: 'Ready', dotClass: 'bg-muted-foreground' },
    loading: { label: 'Loading', dotClass: 'bg-yellow-500 animate-pulse' },
    streaming: { label: 'Streaming', dotClass: 'bg-green-500 animate-pulse' },
    error: { label: 'Error', dotClass: 'bg-red-500' },
};

function ChatHeader({ status }: ChatHeaderProps) {
    const { label, dotClass } = statusConfig[status];

    return (
        <div className="flex items-center justify-center border-b px-4 py-2">
            <div className="flex items-center gap-2 text-muted-foreground">
                <span className={cn('h-1.5 w-1.5 rounded-full', dotClass)} />
                <span className="text-xs">{label}</span>
            </div>
        </div>
    );
}

export { ChatHeader };
