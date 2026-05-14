import type { ProviderType } from '@crewride/core';
import lobiOpenai from '~icons/lobe/openai';
import lobiAnthropic from '~icons/lobe/anthropic';
import lobiGoogle from '~icons/lobe/google';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Icon } from '@/components/ui/icon';
import { cn } from '@/lib/utils';

interface ModelSelectorProps {
    value: string;
    provider: ProviderType;
    onValueChange: (model: string, provider: ProviderType) => void;
    /** 紧凑模式：用于输入栏内嵌 */
    compact?: boolean;
}

interface ModelOption {
    model: string;
    name: string;
    provider: ProviderType;
}

const MODELS: ModelOption[] = [
    { model: 'gpt-4o', name: 'GPT-4o', provider: 'openai' },
    { model: 'gpt-4o-mini', name: 'GPT-4o Mini', provider: 'openai' },
    { model: 'gpt-3.5-turbo', name: 'GPT-3.5 Turbo', provider: 'openai' },
    { model: 'claude-sonnet-4-20250514', name: 'Claude Sonnet 4', provider: 'anthropic' },
    { model: 'claude-3-5-haiku-20241022', name: 'Claude 3.5 Haiku', provider: 'anthropic' },
    { model: 'gemini-2.0-flash', name: 'Gemini 2.0 Flash', provider: 'gemini' },
    { model: 'gemini-2.5-pro-preview-05-06', name: 'Gemini 2.5 Pro', provider: 'gemini' },
];

const PROVIDER_LABELS: Record<ProviderType, string> = {
    openai: 'OpenAI',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
};

const PROVIDER_ICONS: Record<ProviderType, string> = {
    openai: lobiOpenai,
    anthropic: lobiAnthropic,
    gemini: lobiGoogle,
};

function ProviderLogo({ provider, className }: { provider: ProviderType; className?: string }) {
    return <Icon raw={PROVIDER_ICONS[provider]} className={cn('h-3.5 w-3.5 text-xs', className)} />;
}

function ModelSelector({ value, provider, onValueChange, compact }: ModelSelectorProps) {
    const currentValue = `${provider}:${value}`;
    const currentModel = MODELS.find((m) => m.model === value && m.provider === provider);

    const handleChange = (compositeValue: string) => {
        const [prov, ...rest] = compositeValue.split(':');
        const model = rest.join(':');
        onValueChange(model, prov as ProviderType);
    };

    return (
        <Select value={currentValue} onValueChange={handleChange}>
            <SelectTrigger
                className={cn(
                    compact
                        ? 'h-8 w-auto gap-1.5 border-0 bg-transparent px-2 shadow-none hover:bg-accent focus:ring-0'
                        : 'w-full',
                )}
            >
                {compact ? (
                    <>
                        <ProviderLogo provider={provider} />
                        <span className="text-xs font-medium">{currentModel?.name ?? value}</span>
                    </>
                ) : (
                    <SelectValue placeholder="Select a model" />
                )}
            </SelectTrigger>
            <SelectContent>
                {(['openai', 'anthropic', 'gemini'] as ProviderType[]).map((prov) => (
                    <SelectGroup key={prov}>
                        <SelectLabel className="flex items-center gap-1.5">
                            <ProviderLogo provider={prov} />
                            {PROVIDER_LABELS[prov]}
                        </SelectLabel>
                        {MODELS.filter((m) => m.provider === prov).map((opt) => (
                            <SelectItem
                                key={`${opt.provider}:${opt.model}`}
                                value={`${opt.provider}:${opt.model}`}
                            >
                                {opt.name}
                            </SelectItem>
                        ))}
                    </SelectGroup>
                ))}
            </SelectContent>
        </Select>
    );
}

export { ModelSelector, type ModelOption, ProviderLogo };
