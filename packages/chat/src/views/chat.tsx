import { useCallback, useEffect, useMemo, useState } from 'react';
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
import { Badge } from '@/components/badge';
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/collapsible';
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
import { useChat, useModels } from '@crewride/core';
import type { ModelInfo, Message as MessageData } from '@crewride/core';

const suggestions = [
    'What are the latest trends in AI?',
    'How does machine learning work?',
    'Explain quantum computing',
    'Best practices for React development',
    'Tell me about TypeScript benefits',
    'How to optimize database queries?',
];

function PromptAttach() {
    const attachments = usePromptInputAttachments();

    if (attachments.files.length) {
        return (
            <Attachments variant="inline">
                {attachments.files.map((file) => (
                    <Attachment key={file.id} data={file} onRemove={() => attachments.remove(file.id)}>
                        <AttachmentPreview />
                        <AttachmentRemove />
                    </Attachment>
                ))}
            </Attachments>
        );
    } else {
        return (<></>);
    }
}

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
                                        {message.tools?.map((tool, i) => (
                                            <Collapsible key={`${tool.name}-${i}`} className="my-2 rounded-lg border bg-muted/30">
                                                <CollapsibleTrigger className="flex w-full items-center gap-2 px-3 py-2 text-sm">
                                                    <span className="font-medium">{tool.name}</span>
                                                    <Badge variant={
                                                        tool.status === 'output-available' ? 'default' :
                                                        tool.status === 'input-available' ? 'secondary' : 'outline'
                                                    }>
                                                        {tool.error ? 'error' : tool.result ? 'done' : 'pending'}
                                                    </Badge>
                                                </CollapsibleTrigger>
                                                <CollapsibleContent className="space-y-2 px-3 pb-3">
                                                    <div>
                                                        <p className="mb-1 text-xs text-muted-foreground">Parameters</p>
                                                        <pre className="max-h-32 overflow-auto rounded bg-muted p-2 text-xs">
                                                            {JSON.stringify(tool.parameters, null, 2)}
                                                        </pre>
                                                    </div>
                                                    {tool.result && (
                                                        <div>
                                                            <p className="mb-1 text-xs text-muted-foreground">Result</p>
                                                            <pre className="max-h-48 overflow-auto rounded bg-muted p-2 text-xs whitespace-pre-wrap">
                                                                {tool.result}
                                                            </pre>
                                                        </div>
                                                    )}
                                                    {tool.error && (
                                                        <div>
                                                            <p className="mb-1 text-xs text-destructive">Error</p>
                                                            <pre className="max-h-32 overflow-auto rounded bg-destructive/10 p-2 text-xs text-destructive whitespace-pre-wrap">
                                                                {tool.error}
                                                            </pre>
                                                        </div>
                                                    )}
                                                </CollapsibleContent>
                                            </Collapsible>
                                        ))}
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
    onSelect,
}: {
    model: ModelInfo | null;
    onSelect: (modelId: ModelInfo) => void;
}) {
    const { models, refresh } = useModels();
    const [open, setOpen] = useState(false);

    const providers = useMemo(
        () => [...new Set(models.map((m) => m.provider))],
        [models],
    );

    useEffect(() => {
        refresh();
    }, [refresh]);

    useEffect(() => {
        if (!model && models.length) {
            onSelect(models[0]);
        }
    }, [model, models]);

    return (
        <ModelSelector onOpenChange={setOpen} open={open}>
            <ModelSelectorTrigger asChild>
                <PromptInputButton>
                    {model?.provider && <ModelSelectorLogo provider={model.provider} />}
                    {model?.name && <ModelSelectorName>{model.name}</ModelSelectorName>}
                </PromptInputButton>
            </ModelSelectorTrigger>
            <ModelSelectorContent>
                <ModelSelectorInput placeholder="Search models..." />
                <ModelSelectorList>
                    <ModelSelectorEmpty>No models found.</ModelSelectorEmpty>
                    {providers.map((kind) => (
                        <ModelSelectorGroup heading={kind} key={kind}>
                            {models
                                .filter((m) => m.provider === kind)
                                .map((m) => (
                                    <ModelSelectorItem
                                        key={m.model}
                                        value={m.model}
                                        onSelect={() => {
                                            setOpen(false);
                                            onSelect(m)
                                        }}
                                    >
                                        <ModelSelectorLogo provider={m.provider} />
                                        <ModelSelectorName>{m.name}</ModelSelectorName>
                                        <ModelSelectorLogoGroup>
                                            <ModelSelectorLogo key={m.provider} provider={m.provider} />
                                        </ModelSelectorLogoGroup>
                                        {model?.model === m.model ? (
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

export default function ChatView({ sessionId }: { sessionId?: string | null }) {
    const [activeModel, setActiveModel] = useState<ModelInfo | null>(null);
    const [speechText, setSpeechText] = useState('');
    const [useSearch, setUseSearch] = useState(false);

    const { messages, status, sendMessage } = useChat(sessionId);

    // 发送消息
    const handleSubmit = useCallback(
        (message: PromptInputMessage) => {
            const hasText = Boolean(message.text?.trim());
            if ((!hasText && !message.files?.length) || !activeModel) return;

            if (message.files?.length) {
                toast.success('Files attached', {
                    description: `${message.files.length} file(s) attached to message`,
                });
            }

            sendMessage(activeModel.provider, activeModel.model, message.text || 'Sent with attachments');
            setSpeechText('');
        },
        [activeModel, sendMessage],
    );

    const handleSuggest = useCallback(
        (suggestion: string) => {
            if (!activeModel) return;
            sendMessage(activeModel.provider, activeModel.model, suggestion)
        },
        [activeModel, sendMessage],
    );

    const handleModelSelect = useCallback((model: ModelInfo) => {
        setActiveModel(model);
    }, []);

    const submitDisabled = useMemo(
        () => !speechText.trim() || status === 'streaming',
        [speechText, status],
    );

    return (
        <div className="relative flex size-full flex-col divide-y overflow-hidden">
            <MessageList messages={messages} />

            <div className="grid shrink-0 gap-4 pt-4">
                <Suggestions className="px-4">
                    {suggestions.map((suggestion) => (
                        <Suggestion
                            key={suggestion}
                            onClick={handleSuggest}
                            suggestion={suggestion}
                        />
                    ))}
                </Suggestions>
                <div className="w-full px-4 pb-4">
                    <PromptInput globalDrop multiple onSubmit={handleSubmit}>
                        <PromptInputHeader>
                            <PromptAttach />
                        </PromptInputHeader>
                        <PromptInputBody>
                            <PromptInputTextarea
                                onChange={(e) => setSpeechText(e.target.value)}
                                value={speechText}
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
                                    onClick={() => setUseSearch((v) => !v)}
                                    variant={useSearch ? 'default' : 'ghost'}
                                >
                                    <GlobeIcon size={16} />
                                    <span>Search</span>
                                </PromptInputButton>
                                <ModelPicker
                                    model={activeModel}
                                    onSelect={handleModelSelect}
                                />
                            </PromptInputTools>
                            <div className="flex min-w-0 items-center gap-1">
                                <SpeechInput
                                    className="shrink-0"
                                    onTranscriptionChange={(t) =>
                                        setSpeechText((prev) => (prev ? `${prev} ${t}` : t))
                                    }
                                    size="sm"
                                    variant="ghost"
                                />
                                <PromptInputSubmit
                                    disabled={submitDisabled}
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
