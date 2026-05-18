import { useCallback, useMemo, useState } from 'react';
import { CheckIcon, GlobeIcon } from 'lucide-react';
import { toast } from 'sonner';
import type { AttachmentData } from '@/components/attachments';
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
import type { MessageType } from '@crewride/core';
import {
    initMessages,
    models,
    chefs,
    suggestions,
    mockResponses,
    delay,
} from '@crewride/core/mocks';

const AttachmentItem = ({
    attachment,
    onRemove,
}: {
    attachment: AttachmentData;
    onRemove: (id: string) => void;
}) => {
    const handleRemove = useCallback(() => {
        onRemove(attachment.id);
    }, [onRemove, attachment.id]);

    return (
        <Attachment data={attachment} onRemove={handleRemove}>
            <AttachmentPreview />
            <AttachmentRemove />
        </Attachment>
    );
};

const PromptInputAttachmentsDisplay = () => {
    const attachments = usePromptInputAttachments();

    const handleRemove = useCallback(
        (id: string) => {
            attachments.remove(id);
        },
        [attachments],
    );

    if (attachments.files.length === 0) {
        return null;
    }

    return (
        <Attachments variant="inline">
            {attachments.files.map((attachment) => (
                <AttachmentItem
                    attachment={attachment}
                    key={attachment.id}
                    onRemove={handleRemove}
                />
            ))}
        </Attachments>
    );
};

const SuggestionItem = ({
    suggestion,
    onClick,
}: {
    suggestion: string;
    onClick: (suggestion: string) => void;
}) => {
    const handleClick = useCallback(() => {
        onClick(suggestion);
    }, [onClick, suggestion]);

    return <Suggestion onClick={handleClick} suggestion={suggestion} />;
};

const ModelItem = ({
    m,
    isSelected,
    onSelect,
}: {
    m: (typeof models)[0];
    isSelected: boolean;
    onSelect: (id: string) => void;
}) => {
    const handleSelect = useCallback(() => {
        onSelect(m.id);
    }, [onSelect, m.id]);

    return (
        <ModelSelectorItem onSelect={handleSelect} value={m.id}>
            <ModelSelectorLogo provider={m.chefSlug} />
            <ModelSelectorName>{m.name}</ModelSelectorName>
            <ModelSelectorLogoGroup>
                {m.providers.map((provider) => (
                    <ModelSelectorLogo key={provider} provider={provider} />
                ))}
            </ModelSelectorLogoGroup>
            {isSelected ? (
                <CheckIcon className="ml-auto size-4" />
            ) : (
                <div className="ml-auto size-4" />
            )}
        </ModelSelectorItem>
    );
};

export default function ChatView() {
    const [model, setModel] = useState<string>(models[0].id);
    const [modelSelectorOpen, setModelSelectorOpen] = useState(false);
    const [text, setText] = useState<string>('');
    const [useWebSearch, setUseWebSearch] = useState<boolean>(false);
    const [status, setStatus] = useState<'submitted' | 'streaming' | 'ready' | 'error'>('ready');
    const [messages, setMessages] = useState<MessageType[]>(initMessages);
    const [, setStreamingMessageId] = useState<string | null>(null);

    const selectedModelData = useMemo(() => models.find((m) => m.id === model), [model]);

    const updateMessageContent = useCallback((messageId: string, newContent: string) => {
        setMessages((prev) =>
            prev.map((msg) => {
                if (msg.versions.some((v) => v.id === messageId)) {
                    return {
                        ...msg,
                        versions: msg.versions.map((v) =>
                            v.id === messageId ? { ...v, content: newContent } : v,
                        ),
                    };
                }
                return msg;
            }),
        );
    }, []);

    const streamResponse = useCallback(
        async (messageId: string, content: string) => {
            setStatus('streaming');
            setStreamingMessageId(messageId);

            const words = content.split(' ');
            let currentContent = '';

            for (const [i, word] of words.entries()) {
                currentContent += (i > 0 ? ' ' : '') + word;
                updateMessageContent(messageId, currentContent);
                await delay(Math.random() * 100 + 50);
            }

            setStatus('ready');
            setStreamingMessageId(null);
        },
        [updateMessageContent],
    );

    const addUserMessage = useCallback(
        (content: string) => {
            const userMessage: MessageType = {
                from: 'user',
                key: `user-${Date.now()}`,
                versions: [
                    {
                        content,
                        id: `user-${Date.now()}`,
                    },
                ],
            };

            setMessages((prev) => [...prev, userMessage]);

            setTimeout(() => {
                const assistantMessageId = `assistant-${Date.now()}`;
                const randomResponse =
                    mockResponses[Math.floor(Math.random() * mockResponses.length)];

                const assistantMessage: MessageType = {
                    from: 'assistant',
                    key: `assistant-${Date.now()}`,
                    versions: [
                        {
                            content: '',
                            id: assistantMessageId,
                        },
                    ],
                };

                setMessages((prev) => [...prev, assistantMessage]);
                streamResponse(assistantMessageId, randomResponse);
            }, 500);
        },
        [streamResponse],
    );

    const handleSubmit = useCallback(
        (message: PromptInputMessage) => {
            const hasText = Boolean(message.text);
            const hasAttachments = Boolean(message.files?.length);

            if (!(hasText || hasAttachments)) {
                return;
            }

            setStatus('submitted');

            if (message.files?.length) {
                toast.success('Files attached', {
                    description: `${message.files.length} file(s) attached to message`,
                });
            }

            addUserMessage(message.text || 'Sent with attachments');
            setText('');
        },
        [addUserMessage],
    );

    const handleSuggestionClick = useCallback(
        (suggestion: string) => {
            setStatus('submitted');
            addUserMessage(suggestion);
        },
        [addUserMessage],
    );

    const handleTranscriptionChange = useCallback((transcript: string) => {
        setText((prev) => (prev ? `${prev} ${transcript}` : transcript));
    }, []);

    const handleTextChange = useCallback((event: React.ChangeEvent<HTMLTextAreaElement>) => {
        setText(event.target.value);
    }, []);

    const toggleWebSearch = useCallback(() => {
        setUseWebSearch((prev) => !prev);
    }, []);

    const handleModelSelect = useCallback((modelId: string) => {
        setModel(modelId);
        setModelSelectorOpen(false);
    }, []);

    const isSubmitDisabled = useMemo(
        () => !(text.trim() || status) || status === 'streaming',
        [text, status],
    );

    return (
        <div className="relative flex size-full flex-col divide-y overflow-hidden">
            <Conversation>
                <ConversationContent>
                    {messages.map(({ versions, ...message }) => (
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
                                                    <SourcesTrigger
                                                        count={message.sources.length}
                                                    />
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
            <div className="grid shrink-0 gap-4 pt-4">
                <Suggestions className="px-4">
                    {suggestions.map((suggestion) => (
                        <SuggestionItem
                            key={suggestion}
                            onClick={handleSuggestionClick}
                            suggestion={suggestion}
                        />
                    ))}
                </Suggestions>
                <div className="w-full px-4 pb-4">
                    <PromptInput globalDrop multiple onSubmit={handleSubmit}>
                        <PromptInputHeader>
                            <PromptInputAttachmentsDisplay />
                        </PromptInputHeader>
                        <PromptInputBody>
                            <PromptInputTextarea onChange={handleTextChange} value={text} />
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
                                    onClick={toggleWebSearch}
                                    variant={useWebSearch ? 'default' : 'ghost'}
                                >
                                    <GlobeIcon size={16} />
                                    <span>Search</span>
                                </PromptInputButton>
                                <ModelSelector
                                    onOpenChange={setModelSelectorOpen}
                                    open={modelSelectorOpen}
                                >
                                    <ModelSelectorTrigger asChild>
                                        <PromptInputButton>
                                            {selectedModelData?.chefSlug && (
                                                <ModelSelectorLogo
                                                    provider={selectedModelData.chefSlug}
                                                />
                                            )}
                                            {selectedModelData?.name && (
                                                <ModelSelectorName>
                                                    {selectedModelData.name}
                                                </ModelSelectorName>
                                            )}
                                        </PromptInputButton>
                                    </ModelSelectorTrigger>
                                    <ModelSelectorContent>
                                        <ModelSelectorInput placeholder="Search models..." />
                                        <ModelSelectorList>
                                            <ModelSelectorEmpty>
                                                No models found.
                                            </ModelSelectorEmpty>
                                            {chefs.map((chef) => (
                                                <ModelSelectorGroup heading={chef} key={chef}>
                                                    {models
                                                        .filter((m) => m.chef === chef)
                                                        .map((m) => (
                                                            <ModelItem
                                                                isSelected={model === m.id}
                                                                key={m.id}
                                                                m={m}
                                                                onSelect={handleModelSelect}
                                                            />
                                                        ))}
                                                </ModelSelectorGroup>
                                            ))}
                                        </ModelSelectorList>
                                    </ModelSelectorContent>
                                </ModelSelector>
                            </PromptInputTools>
                            <div className="flex min-w-0 items-center gap-1">
                                <SpeechInput
                                    className="shrink-0"
                                    onTranscriptionChange={handleTranscriptionChange}
                                    size="sm"
                                    variant="ghost"
                                />
                                <PromptInputSubmit disabled={isSubmitDisabled} status={status} />
                            </div>
                        </PromptInputFooter>
                    </PromptInput>
                </div>
            </div>
        </div>
    );
}
