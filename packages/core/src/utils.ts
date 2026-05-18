export function formatDuration(seconds: number): string {
    if (seconds < 60) return `${Math.round(seconds)}s`;
    const mins = Math.floor(seconds / 60);
    const secs = Math.round(seconds % 60);
    return secs > 0 ? `${mins}m${secs}s` : `${mins}m`;
}

export function truncateText(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text;
    return `${text.slice(0, maxLength).trim()}...`;
}

export function generateTitleFromMessage(content: string): string {
    const cleaned = content.replace(/[#*`>~\[\]()]+/g, '').trim();
    const firstLine = cleaned.split('\n')[0];
    return truncateText(firstLine, 50) || 'New Chat';
}