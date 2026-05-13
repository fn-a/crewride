import { useState, useCallback, useRef, useEffect, type KeyboardEvent } from 'react';
import type { ChatStatus, ProviderType } from '@crewride/core';
import mdiPaperclip from '~icons/mdi/paperclip';
import mdiChevronDown from '~icons/mdi/chevron-down';
import mdiCheck from '~icons/mdi/check';
import mdiStop from '~icons/mdi/stop';
import mdiArrowUp from '~icons/mdi/arrow-up';
import mdiCreationOutline from '~icons/mdi/creation-outline';
import mdiHeadLightbulbOutline from '~icons/mdi/head-lightbulb-outline';
import mdiLightningBolt from '~icons/mdi/lightning-bolt';
import { Button } from '@/components/ui/button';
import { Icon } from '@/components/ui/icon';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ModelSelector } from '@/components/provider/model-selector';
import { cn } from '@/lib/utils';

type ChatMode = 'auto' | 'think' | 'fast';

interface ChatInputProps {
    onSend: (content: string) => void;
    status: ChatStatus;
    onStop: () => void;
    model: string;
    provider: ProviderType;
    onModelChange: (model: string, provider: ProviderType) => void;
}

const MODES: { value: ChatMode; label: string; icon: string; description: string }[] = [
    { value: 'auto', label: 'Auto', icon: mdiCreationOutline, description: 'Balanced response' },
    {
        value: 'think',
        label: 'Think',
        icon: mdiHeadLightbulbOutline,
        description: 'Deep reasoning',
    },
    { value: 'fast', label: 'Fast', icon: mdiLightningBolt, description: 'Quick response' },
];

function ChatInput({ onSend, status, onStop, model, provider, onModelChange }: ChatInputProps) {
    const [input, setInput] = useState('');
    const [mode, setMode] = useState<ChatMode>('auto');
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const isStreaming = status === 'streaming' || status === 'loading';

    const currentMode = MODES.find((m) => m.value === mode)!;

    // 动态高度
    const adjustHeight = useCallback(() => {
        const el = textareaRef.current;
        if (!el) return;
        el.style.height = 'auto';
        el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
    }, []);

    useEffect(() => {
        adjustHeight();
    }, [input, adjustHeight]);

    const handleSend = useCallback(() => {
        const trimmed = input.trim();
        if (!trimmed || isStreaming) return;
        onSend(trimmed);
        setInput('');
        requestAnimationFrame(() => {
            if (textareaRef.current) {
                textareaRef.current.style.height = 'auto';
            }
        });
    }, [input, isStreaming, onSend]);

    const handleKeyDown = useCallback(
        (e: KeyboardEvent<HTMLTextAreaElement>) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSend();
            }
        },
        [handleSend],
    );

    return (
        <div className="border-t bg-background p-4">
            <div className="mx-auto w-full max-w-3xl">
                {/* 输入区域容器 */}
                <div className="rounded-xl border bg-background shadow-sm transition-shadow focus-within:shadow-md focus-within:ring-1 focus-within:ring-ring">
                    {/* 文本输入 */}
                    <textarea
                        ref={textareaRef}
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder="Send a message..."
                        disabled={isStreaming}
                        rows={1}
                        className={cn(
                            'w-full resize-none bg-transparent px-4 pt-3.5 pb-1 text-sm outline-none',
                            'placeholder:text-muted-foreground',
                            'max-h-50 overflow-y-auto',
                            'disabled:cursor-not-allowed disabled:opacity-50',
                        )}
                    />

                    {/* 底部工具栏 */}
                    <div className="flex items-center justify-between px-3 pb-2.5 pt-1">
                        {/* 左侧：附件 + 模式切换 + 模型选择 */}
                        <div className="flex items-center gap-1">
                            {/* 附件按钮 */}
                            <Button
                                variant="ghost"
                                size="icon"
                                className="h-8 w-8 text-muted-foreground hover:text-foreground"
                            >
                                <Icon raw={mdiPaperclip} className="h-4 w-4" />
                            </Button>

                            {/* 分隔线 */}
                            <div className="mx-0.5 h-4 w-px bg-border" />

                            {/* 模式下拉 */}
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        className="h-8 gap-1.5 px-2 text-muted-foreground hover:text-foreground"
                                    >
                                        <Icon raw={currentMode.icon} className="h-3.5 w-3.5" />
                                        <span className="text-xs font-medium">
                                            {currentMode.label}
                                        </span>
                                        <Icon raw={mdiChevronDown} className="h-3 w-3 opacity-50" />
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="start" className="w-52">
                                    {MODES.map((m) => (
                                        <DropdownMenuItem
                                            key={m.value}
                                            onClick={() => setMode(m.value)}
                                            className={cn(
                                                'flex items-center gap-2',
                                                mode === m.value && 'bg-accent',
                                            )}
                                        >
                                            <Icon raw={m.icon} className="h-4 w-4" />
                                            <div className="flex flex-col">
                                                <span className="text-sm font-medium">
                                                    {m.label}
                                                </span>
                                                <span className="text-[11px] text-muted-foreground">
                                                    {m.description}
                                                </span>
                                            </div>
                                            {mode === m.value && (
                                                <Icon raw={mdiCheck} className="ml-auto h-4 w-4" />
                                            )}
                                        </DropdownMenuItem>
                                    ))}
                                </DropdownMenuContent>
                            </DropdownMenu>

                            {/* 分隔线 */}
                            <div className="mx-0.5 h-4 w-px bg-border" />

                            {/* 模型选择 */}
                            <ModelSelector
                                value={model}
                                provider={provider}
                                onValueChange={onModelChange}
                                compact
                            />
                        </div>

                        {/* 右侧：发送/停止按钮 */}
                        {isStreaming ? (
                            <Button
                                variant="destructive"
                                size="sm"
                                onClick={onStop}
                                className="h-8 gap-1.5 rounded-lg"
                            >
                                <Icon raw={mdiStop} className="h-3.5 w-3.5" />
                                Stop
                            </Button>
                        ) : (
                            <Button
                                size="sm"
                                onClick={handleSend}
                                disabled={!input.trim()}
                                className={cn(
                                    'h-8 gap-1.5 rounded-lg transition-all',
                                    input.trim()
                                        ? 'bg-primary text-primary-foreground shadow-sm'
                                        : 'bg-muted text-muted-foreground',
                                )}
                            >
                                <Icon raw={mdiArrowUp} className="h-4 w-4" />
                                Send
                            </Button>
                        )}
                    </div>
                </div>

                {/* 底部提示 */}
                <p className="mt-2 text-center text-[11px] text-muted-foreground">
                    Enter to send · Shift + Enter for new line
                </p>
            </div>
        </div>
    );
}

export { ChatInput, type ChatMode };
