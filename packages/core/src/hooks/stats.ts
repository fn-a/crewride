import { useState, useCallback } from 'react';
import type { TokenUsage } from '../types';
import { BACKEND_URL } from '../config';

// 查询当前 token 用量统计
export async function fetchStats(): Promise<TokenUsage> {
    const response = await fetch(`${BACKEND_URL}/stats`);
    if (!response.ok) {
        throw new Error(`Failed to fetch stats: ${response.status}`);
    }
    return response.json();
}


// 查询后端 /stats 端点获取累计 token 统计
export function useStats() {
    const [stats, setStats] = useState<TokenUsage | null>(null);
    const [loading, setLoading] = useState(false);

    const refresh = useCallback(async () => {
        setLoading(true);
        try {
            const data = await fetchStats();
            setStats(data);
        } catch (e) {
            console.error('Failed to fetch stats:', e);
        } finally {
            setLoading(false);
        }
    }, []);

    return { stats, loading, refresh };
}
