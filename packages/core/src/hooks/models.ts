import { useState, useCallback } from 'react';
import type { ModelInfo } from '../types';
import { BASE_URL } from '../config';

const MODELS_API = '/api/models';

// 查询后端 /models 端点获取模型列表
export function useModels() {
    const [models, setModels] = useState<ModelInfo[]>([]);
    const [loading, setLoading] = useState(false);

    const refresh = useCallback(async () => {
        setLoading(true);
        try {
            const listApi = new URL(MODELS_API, BASE_URL);
            const response = await fetch(listApi);
            if (!response.ok) {
                throw new Error(`Failed to fetch models: ${response.status}`);
            }
            const data = await response.json();
            // TODO 返回数据结构校验
            setModels(data);
        } catch (e) {
            console.error('Failed to fetch models:', e);
        } finally {
            setLoading(false);
        }
    }, []);

    return { models, loading, refresh };
}
