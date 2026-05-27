import { useCallback, useMemo, useState } from 'react';
import { CheckIcon, GlobeIcon } from 'lucide-react';
import { toast } from 'sonner';
import {
    Attachment,
    AttachmentPreview,
    AttachmentRemove,
    Attachments,
} from '@/components/attachments';
import {
    Conversation,
    ConversationContent,
    ConversationScrollButton,
} from '@/components/conversation';
import {
    Message,
    MessageBranch,
    MessageBranchContent,
    MessageBranchNext,
    MessageBranchPage,
    MessageBranchPrevious,
    MessageBranchSelector,
    MessageContent,
    MessageResponse,
} from '@/components/message';
import {
    ModelSelector,
    ModelSelectorContent,
    ModelSelectorEmpty,
    ModelSelectorGroup,
    ModelSelectorInput,
    ModelSelectorItem,
    ModelSelectorList,
    ModelSelectorLogo,
    ModelSelectorLogoGroup,
    ModelSelectorName,
    ModelSelectorTrigger,
} from '@/components/model-selector';
import type { PromptInputMessage } from '@/components/prompt-input';
import {
    PromptInput,
    PromptInputActionAddAttachments,
    PromptInputActionMenu,
    PromptInputActionMenuContent,
    PromptInputActionMenuTrigger,
    PromptInputBody,
    PromptInputButton,
    PromptInputFooter,
    PromptInputHeader,
    PromptInputSubmit,
    PromptInputTextarea,
    PromptInputTools,
    usePromptInputAttachments,
} from '@/components/prompt-input';
import { Reasoning, ReasoningContent, ReasoningTrigger } from '@/components/reasoning';
import { Source, Sources, SourcesContent, SourcesTrigger } from '@/components/sources';
import { SpeechInput } from '@/components/speech-input';
import { Suggestion, Suggestions } from '@/components/suggestion';
import { useChat, type ProviderKind, type Message as MessageData } from '@crewride/core';

// 模型列表
const models = [
    { chef: 'OpenAI', slug: 'openai', id: 'gpt-4o', name: 'GPT-4o', providers: ['openai', 'azure'] },
    { chef: 'OpenAI', slug: 'openai', id: 'gpt-4o-mini', name: 'GPT-4o Mini', providers: ['openai', 'azure'] },
    { chef: 'Anthropic', slug: 'anthropic', id: 'claude-sonnet-4-20250514', name: 'Claude 4 Sonnet', providers: ['anthropic', 'azure', 'google', 'amazon-bedrock'] },
    { chef: 'Anthropic', slug: 'anthropic', id: 'claude-opus-4-20250514', name: 'Claude 4 Opus', providers: ['anthropic', 'azure', 'google', 'amazon-bedrock'] },
    { chef: 'Google', slug: 'gemini', id: 'gemini-2.0-flash-exp', name: 'Gemini 2.0 Flash', providers: ['google'] },
];

const chefs = ['OpenAI', 'Anthropic', 'Google'];

const suggestions = [
    'What are the latest trends in AI?',
    'How does machine learning work?',
    'Explain quantum computing',
    'Best practices for React development',
    'Tell me about TypeScript benefits',
    'How to optimize database queries?',
];

// 消息列表
function MessageList({ messages }: { messages: MessageData[] }) {
    return (
        <Conversation>
            <ConversationContent>
                {messages.map(({ versions, ...message }: MessageData) => (
                    <MessageBranch defaultBranch={0} key={message.key}>
                        <MessageBranchContent>
                            {versions.map((version) => (
                                <Message
                                    from={message.from}
                                    key={`${message.key}-${version.id}`}
                                >
                                    <div>
                                        {message.sources?.length && (
                                            <Sources>
                                                <SourcesTrigger count={message.sources.length} />
                                                <SourcesContent>
                                                    {message.sources.map((source) => (
                                                        <Source
                                                            href={source.href}
                                                            key={source.href}
                                                            title={source.title}
                                                        />
                                                    ))}
                                                </SourcesContent>
                                            </Sources>
                                        )}
                                        {message.reasoning && (
                                            <Reasoning duration={message.reasoning.duration}>
                                                <ReasoningTrigger />
                                                <ReasoningContent>
                                                    {message.reasoning.content}
                                                </ReasoningContent>
                                            </Reasoning>
                                        )}
                                        <MessageContent>
                                            <MessageResponse>{version.content}</MessageResponse>
                                        </MessageContent>
                                    </div>
                                </Message>
                            ))}
                        </MessageBranchContent>
                        {versions.length > 1 && (
                            <MessageBranchSelector>
                                <MessageBranchPrevious />
                                <MessageBranchPage />
                                <MessageBranchNext />
                            </MessageBranchSelector>
                        )}
                    </MessageBranch>
                ))}
            </ConversationContent>
            <ConversationScrollButton />
        </Conversation>
    );
}

// 模型选择
function ModelPicker({
    model,
    open,
    onOpenChange,
    onSelect,
}: {
    model: string;
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onSelect: (modelId: string) => void;
}) {
    const selected = useMemo(() => models.find((m) => m.id === model), [model]);

    return (
        <ModelSelector onOpenChange={onOpenChange} open={open}>
            <ModelSelectorTrigger asChild>
                <PromptInputButton>
                    {selected?.slug && <ModelSelectorLogo provider={selected.slug} />}
                    {selected?.name && <ModelSelectorName>{selected.name}</ModelSelectorName>}
                </PromptInputButton>
            </ModelSelectorTrigger>
            <ModelSelectorContent>
                <ModelSelectorInput placeholder="Search models..." />
                <ModelSelectorList>
                    <ModelSelectorEmpty>No models found.</ModelSelectorEmpty>
                    {chefs.map((chef) => (
                        <ModelSelectorGroup heading={chef} key={chef}>
                            {models
                                .filter((m) => m.chef === chef)
                                .map((m) => (
                                    <ModelSelectorItem
                                        key={m.id}
                                        value={m.id}
                                        onSelect={() => onSelect(m.id)}
                                    >
                                        <ModelSelectorLogo provider={m.slug} />
                                        <ModelSelectorName>{m.name}</ModelSelectorName>
                                        <ModelSelectorLogoGroup>
                                            {m.providers.map((p) => (
                                                <ModelSelectorLogo key={p} provider={p} />
                                            ))}
                                        </ModelSelectorLogoGroup>
                                        {model === m.id ? (
                                            <CheckIcon className="ml-auto size-4" />
                                        ) : (
                                            <div className="ml-auto size-4" />
                                        )}
                                    </ModelSelectorItem>
                                ))}
                        </ModelSelectorGroup>
                    ))}
                </ModelSelectorList>
            </ModelSelectorContent>
        </ModelSelector>
    );
}

export default function ChatView() {
    const [model, setModel] = useState(models[0].id);
    const [modelSelectorOpen, setModelSelectorOpen] = useState(false);
    const [text, setText] = useState('');
    const [useWebSearch, setUseWebSearch] = useState(false);

    const { messages, status, sendMessage } = useChat();

    const selectedModelData = useMemo(() => models.find((m) => m.id === model), [model]);

    const providerKind: ProviderKind = useMemo(
        () => (selectedModelData?.slug as ProviderKind) ?? 'openai',
        [selectedModelData],
    );

    // 发送消息
    const handleSubmit = useCallback(
        (message: PromptInputMessage) => {
            const hasText = Boolean(message.text?.trim());
            if (!hasText && !message.files?.length) return;

            if (message.files?.length) {
                toast.success('Files attached', {
                    description: `${message.files.length} file(s) attached to message`,
                });
            }

            sendMessage(providerKind, model, message.text || 'Sent with attachments');
            setText('');
        },
        [providerKind, model, sendMessage],
    );

    const handleSuggestionClick = useCallback(
        (suggestion: string) => sendMessage(providerKind, model, suggestion),
        [providerKind, model, sendMessage],
    );

    const handleModelSelect = useCallback((modelId: string) => {
        setModel(modelId);
        setModelSelectorOpen(false);
    }, []);

    const attachments = usePromptInputAttachments();

    const isSubmitDisabled = useMemo(
        () => !text.trim() || status === 'streaming',
        [text, status],
    );

    return (
        <div className="relative flex size-full flex-col divide-y overflow-hidden">
            <MessageList messages={messages} />

            <div className="grid shrink-0 gap-4 pt-4">
                <Suggestions className="px-4">
                    {suggestions.map((suggestion) => (
                        <Suggestion
                            key={suggestion}
                            onClick={handleSuggestionClick}
                            suggestion={suggestion}
                        />
                    ))}
                </Suggestions>
                <div className="w-full px-4 pb-4">
                    <PromptInput globalDrop multiple onSubmit={handleSubmit}>
                        <PromptInputHeader>
                            {attachments.files.length > 0 && (
                                <Attachments variant="inline">
                                    {attachments.files.map((file) => (
                                        <Attachment key={file.id} data={file} onRemove={() => attachments.remove(file.id)}>
                                            <AttachmentPreview />
                                            <AttachmentRemove />
                                        </Attachment>
                                    ))}
                                </Attachments>
                            )}
                        </PromptInputHeader>
                        <PromptInputBody>
                            <PromptInputTextarea
                                onChange={(e) => setText(e.target.value)}
                                value={text}
                            />
                        </PromptInputBody>
                        <PromptInputFooter>
                            <PromptInputTools>
                                <PromptInputActionMenu>
                                    <PromptInputActionMenuTrigger />
                                    <PromptInputActionMenuContent>
                                        <PromptInputActionAddAttachments />
                                    </PromptInputActionMenuContent>
                                </PromptInputActionMenu>
                                <PromptInputButton
                                    onClick={() => setUseWebSearch((v) => !v)}
                                    variant={useWebSearch ? 'default' : 'ghost'}
                                >
                                    <GlobeIcon size={16} />
                                    <span>Search</span>
                                </PromptInputButton>
                                <ModelPicker
                                    model={model}
                                    open={modelSelectorOpen}
                                    onOpenChange={setModelSelectorOpen}
                                    onSelect={handleModelSelect}
                                />
                            </PromptInputTools>
                            <div className="flex min-w-0 items-center gap-1">
                                <SpeechInput
                                    className="shrink-0"
                                    onTranscriptionChange={(t) =>
                                        setText((prev) => (prev ? `${prev} ${t}` : t))
                                    }
                                    size="sm"
                                    variant="ghost"
                                />
                                <PromptInputSubmit
                                    disabled={isSubmitDisabled}
                                    status={status}
                                />
                            </div>
                        </PromptInputFooter>
                    </PromptInput>
                </div>
            </div>
        </div>
    );
}
