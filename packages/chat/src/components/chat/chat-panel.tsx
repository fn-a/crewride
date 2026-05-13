import type { ChatSession, ChatStatus, ProviderType } from '@crewride/core';
import { ChatHeader } from './chat-header';
import { ChatMessages } from './chat-messages';
import { ChatInput } from './chat-input';

interface ChatPanelProps {
    session: ChatSession | null;
    status: ChatStatus;
    onSend: (content: string) => void;
    onStop: () => void;
    onModelChange: (model: string, provider: ProviderType) => void;
}

function ChatPanel({ session, status, onSend, onStop, onModelChange }: ChatPanelProps) {
    if (!session) {
        return (
            <div className="flex flex-1 flex-col items-center justify-center gap-4 text-muted-foreground">
                <h2 className="text-2xl font-bold text-foreground">CrewRide Chat</h2>
                <p className="text-sm">Select a model and start a new conversation</p>
            </div>
        );
    }

    return (
        <div className="flex flex-1 flex-col">
            <ChatHeader status={status} />
            <ChatMessages messages={session.messages} provider={session.provider} />
            <ChatInput
                onSend={onSend}
                status={status}
                onStop={onStop}
                model={session.model}
                provider={session.provider}
                onModelChange={onModelChange}
            />
        </div>
    );
}

export { ChatPanel };
