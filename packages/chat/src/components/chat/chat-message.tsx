import type { Message, ProviderType } from '@crewride/core';
import { ProviderLogo } from '@/components/provider/model-selector';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import mdiAccountOutline from '~icons/mdi/account-outline';
import { Icon } from '@/components/ui/icon';
import { cn } from '@/lib/utils';

interface ChatMessageProps {
    message: Message;
    provider?: ProviderType;
}

function ChatMessage({ message, provider }: ChatMessageProps) {
    const isUser = message.role === 'user';
    const isStreaming = 'isStreaming' in message && message.isStreaming === true;

    return (
        <div className={cn('flex gap-3 px-4 py-3', isUser ? 'justify-end' : 'justify-start')}>
            {!isUser && (
                <Avatar className="h-8 w-8 shrink-0">
                    <AvatarFallback className="bg-background p-1.5">
                        <ProviderLogo provider={provider ?? 'openai'} className="h-5 w-5" />
                    </AvatarFallback>
                </Avatar>
            )}

            <div
                className={cn(
                    'max-w-[75%] rounded-lg px-3 py-2 text-sm',
                    isUser ? 'bg-primary text-primary-foreground' : 'bg-muted text-foreground',
                )}
            >
                <div className="whitespace-pre-wrap wrap-break-word">
                    {message.content || (
                        <span className="inline-block h-4 w-1 animate-pulse bg-current" />
                    )}
                </div>
                {isStreaming && (
                    <span className="inline-block h-4 w-1 animate-pulse bg-current ml-0.5" />
                )}
            </div>

            {isUser && (
                <Avatar className="h-8 w-8 shrink-0">
                    <AvatarFallback className="bg-secondary text-secondary-foreground">
                        <Icon raw={mdiAccountOutline} className="h-4 w-4" />
                    </AvatarFallback>
                </Avatar>
            )}
        </div>
    );
}

export { ChatMessage };
