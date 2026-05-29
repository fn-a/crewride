import { createOpenAI } from '@ai-sdk/openai';
import { createAnthropic } from '@ai-sdk/anthropic';
import { createGoogleGenerativeAI } from '@ai-sdk/google';
import type { LanguageModel } from 'ai';
import type { ProviderKind } from '../types';
import { BASE_URL } from '../config';

const openai = createOpenAI({
    baseURL: new URL('/v1', BASE_URL).href,
    apiKey: 'proxy',
});

const anthropic = createAnthropic({
    baseURL: new URL('/v1', BASE_URL).href,
    apiKey: 'proxy',
});

const gemini = createGoogleGenerativeAI({
    baseURL: new URL('/v1beta', BASE_URL).href,
    apiKey: 'proxy',
});

// 根据 Provider 类型和模型 ID 获取 LanguageModel 实例
export function useProvider(kind: ProviderKind, modelId: string): LanguageModel {
    switch (kind) {
        case 'openai':
            return openai.chat(modelId);
        case 'anthropic':
            return anthropic.chat(modelId);
        case 'gemini':
            return gemini.chat(modelId);
    }
}