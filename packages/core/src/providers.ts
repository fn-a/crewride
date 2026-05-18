import { createOpenAI } from '@ai-sdk/openai';
import { createAnthropic } from '@ai-sdk/anthropic';
import { createGoogleGenerativeAI } from '@ai-sdk/google';
import type { ProviderConfig, ProviderKind } from './types';

function createProvider(kind: ProviderKind, config?: ProviderConfig) {
    switch (kind) {
        case 'openai':
            return createOpenAI(config);
        case 'anthropic':
            return createAnthropic(config);
        case 'google':
            return createGoogleGenerativeAI(config);
    }
}

export function getModel(
    kind: ProviderKind,
    modelId: string,
    config?: ProviderConfig,
) {
    const provider = createProvider(kind, config);
    return provider(modelId);
}