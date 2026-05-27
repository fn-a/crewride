import { useEffect, useState } from 'react';
import { Label } from '@/components/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/select';
import { Switch } from '@/components/switch';
import { Separator } from '@/components/separator';
import { useStats } from '@crewride/core';

export default function SettingsView() {
    const [language, setLanguage] = useState('zh');
    const [streamEnabled, setStreamEnabled] = useState(true);
    const [autoScroll, setAutoScroll] = useState(true);

    // 从后端查询统计数据
    const { stats, loading, refresh } = useStats();

    useEffect(() => {
        refresh();
    }, [refresh]);

    return (
        <div className="flex-1 overflow-y-auto p-6">
            <div className="mx-auto max-w-2xl space-y-6">
                {/* 通用设置 */}
                <Card>
                    <CardHeader>
                        <CardTitle>General</CardTitle>
                        <CardDescription>Configure basic preferences</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        <div className="flex items-center justify-between">
                            <div className="space-y-0.5">
                                <Label>Language</Label>
                                <p className="text-xs text-muted-foreground">
                                    Switch interface display language
                                </p>
                            </div>
                            <Select value={language} onValueChange={setLanguage}>
                                <SelectTrigger className="w-30">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="zh">Chinese</SelectItem>
                                    <SelectItem value="en">English</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                        <Separator />
                        <div className="flex items-center justify-between">
                            <div className="space-y-0.5">
                                <Label>Streaming</Label>
                                <p className="text-xs text-muted-foreground">
                                    Enable streaming text generation
                                </p>
                            </div>
                            <Switch checked={streamEnabled} onCheckedChange={setStreamEnabled} />
                        </div>
                        <Separator />
                        <div className="flex items-center justify-between">
                            <div className="space-y-0.5">
                                <Label>Auto-scroll</Label>
                                <p className="text-xs text-muted-foreground">
                                    Auto-scroll to bottom on new messages
                                </p>
                            </div>
                            <Switch checked={autoScroll} onCheckedChange={setAutoScroll} />
                        </div>
                    </CardContent>
                </Card>

                {/* Token 用量统计 */}
                <Card>
                    <CardHeader>
                        <CardTitle>Usage</CardTitle>
                        <CardDescription>Token consumption across all requests</CardDescription>
                    </CardHeader>
                    <CardContent>
                        {loading && !stats ? (
                            <p className="text-sm text-muted-foreground">Loading...</p>
                        ) : stats ? (
                            <div className="grid grid-cols-2 gap-4 text-sm">
                                <div>
                                    <p className="text-muted-foreground">Total Requests</p>
                                    <p className="text-lg font-semibold">{stats.requests}</p>
                                </div>
                                <div>
                                    <p className="text-muted-foreground">Total Tokens</p>
                                    <p className="text-lg font-semibold">{stats.tokens}</p>
                                </div>
                                <div>
                                    <p className="text-muted-foreground">Prompt Tokens</p>
                                    <p className="text-lg font-semibold">{stats.input_tokens}</p>
                                </div>
                                <div>
                                    <p className="text-muted-foreground">Completion Tokens</p>
                                    <p className="text-lg font-semibold">{stats.output_tokens}</p>
                                </div>
                            </div>
                        ) : (
                            <p className="text-sm text-muted-foreground">
                                Backend unavailable
                            </p>
                        )}
                    </CardContent>
                </Card>

                {/* About */}
                <Card>
                    <CardHeader>
                        <CardTitle>About</CardTitle>
                        <CardDescription>CrewRide AI Chat</CardDescription>
                    </CardHeader>
                    <CardContent>
                        <div className="space-y-2 text-sm text-muted-foreground">
                            <p>Version: 0.0.1</p>
                            <p>
                                A high-performance AI proxy service, supporting multiple AI
                                providers (OpenAI, Anthropic, Gemini) with unified API access
                                and cross-provider forwarding capabilities.
                            </p>
                        </div>
                    </CardContent>
                </Card>
            </div>
        </div>
    );
}
