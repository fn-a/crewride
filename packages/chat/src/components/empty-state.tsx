import * as React from 'react';
import { cn } from '@/utils/style';
import { BotIcon } from 'lucide-react';

interface EmptyStateProps {
    className?: string;
    icon?: React.ReactNode;
    title?: string;
    description?: string;
    children?: React.ReactNode;
}

export function EmptyState({
    className,
    icon,
    title = 'Start a conversation',
    description = 'Select a model and type your first message',
    children,
}: EmptyStateProps) {
    return (
        <div
            className={cn(
                'flex flex-1 flex-col items-center justify-center px-4 py-12',
                className,
            )}
        >
            <div className="mx-auto flex max-w-md flex-col items-center text-center">
                <div className="mb-4 flex size-16 items-center justify-center rounded-2xl bg-muted">
                    {icon ?? <BotIcon className="size-8 text-muted-foreground" />}
                </div>
                <h2 className="text-lg font-semibold">{title}</h2>
                <p className="mt-2 text-sm text-muted-foreground">
                    {description}
                </p>
                {children && <div className="mt-6">{children}</div>}
            </div>
        </div>
    );
}
