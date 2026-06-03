import { useState, useCallback } from 'react';
import type { TokenUsage } from '../types';
import { BASE_URL } from '../config';

const STATS_API = '/api/stats';

// 查询后端 /stats 端点获取累计 token 统计
export function useStats() {
    const [stats, setStats] = useState<TokenUsage | null>(null);
    const [loading, setLoading] = useState(false);

    const refresh = useCallback(async () => {
        setLoading(true);
        try {
            const queryApi = new URL(STATS_API, BASE_URL);
            const response = await fetch(queryApi);
            if (!response.ok) {
                throw new Error(`Failed to fetch stats: ${response.status}`);
            }
            const data = await response.json();
            setStats(data);
        } catch (e) {
            console.error('Failed to fetch stats:', e);
        } finally {
            setLoading(false);
        }
    }, []);

    return { stats, loading, refresh };
}
