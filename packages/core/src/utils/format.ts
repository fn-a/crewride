import type { Message, MessageRole } from '../types';

/** Generate a unique ID for messages */
export function generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/** Create a new Message object */
export function createMessage(role: MessageRole, content: string): Message {
    return {
        id: generateId(),
        role,
        content,
        timestamp: Date.now(),
    };
}

/** Extract the first N words from a message as a title preview */
export function deriveTitle(content: string, maxLength: number = 30): string {
    const trimmed = content.trim().replace(/\n/g, ' ');
    if (trimmed.length <= maxLength) return trimmed;
    return trimmed.slice(0, maxLength) + '…';
}

/** Truncate content for display */
export function truncateContent(content: string, maxLength: number = 200): string {
    if (content.length <= maxLength) return content;
    return content.slice(0, maxLength) + '…';
}
