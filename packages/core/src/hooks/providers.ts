import { createOpenAI } from '@ai-sdk/openai';
import { createAnthropic } from '@ai-sdk/anthropic';
import { createGoogleGenerativeAI } from '@ai-sdk/google';
import type { LanguageModel } from 'ai';
import type { ProviderKind } from '../types';
import { BACKEND_URL } from '../config';

// ============ 三个 Provider，baseURL 统一指向 Rust 后端代理 ============

const openai = createOpenAI({
    baseURL: `${BACKEND_URL}/v1`,
    apiKey: 'proxy',
});

const anthropic = createAnthropic({
    baseURL: `${BACKEND_URL}/v1`,
    apiKey: 'proxy',
});

const google = createGoogleGenerativeAI({
    baseURL: `${BACKEND_URL}/v1beta`,
    apiKey: 'proxy',
});

// 根据 Provider 类型和模型 ID 获取 LanguageModel 实例
export function useProvider(kind: ProviderKind, modelId: string): LanguageModel {
    switch (kind) {
        case 'openai':
            return openai(modelId);
        case 'anthropic':
            return anthropic(modelId);
        case 'gemini':
            return google(modelId);
    }
}