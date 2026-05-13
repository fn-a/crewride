/** Supported AI provider types — maps directly to CrewRide backend endpoints */
export type ProviderType = 'openai' | 'anthropic' | 'gemini';

/** Provider configuration — mirrors Rust backend Config.provider */
export interface ProviderConfig {
    key: string;
    name: string;
    type: ProviderType;
    apiKey?: string;
    apiUrl?: string;
    enabled: boolean;
}

/** Model configuration — mirrors Rust backend Config.model */
export interface ModelConfig {
    model: string;
    name?: string;
    provider?: string;
    replace?: {
        apiKey: boolean;
        model?: string;
    };
}

/** Auth headers for each provider type */
export type ProviderAuthHeaders = {
    openai: { Authorization: string };
    anthropic: { 'x-api-key': string; 'anthropic-version': string };
    gemini: Record<string, never>; // Gemini uses query param
};
