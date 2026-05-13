import { useEffect, useRef } from 'react';
import type { Message, ProviderType } from '@crewride/core';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ChatMessage } from './chat-message';

interface ChatMessagesProps {
    messages: Message[];
    provider?: ProviderType;
}

function ChatMessages({ messages, provider }: ChatMessagesProps) {
    const bottomRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to bottom when new messages arrive
    useEffect(() => {
        bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages]);

    // Filter out system messages from display
    const visibleMessages = messages.filter((m) => m.role !== 'system');

    if (visibleMessages.length === 0) {
        return (
            <div className="flex flex-1 items-center justify-center text-muted-foreground">
                <p className="text-sm">Send a message to start chatting</p>
            </div>
        );
    }

    return (
        <ScrollArea className="flex-1">
            <div className="flex flex-col">
                {visibleMessages.map((message) => (
                    <ChatMessage key={message.id} message={message} provider={provider} />
                ))}
                <div ref={bottomRef} />
            </div>
        </ScrollArea>
    );
}

export { ChatMessages };
