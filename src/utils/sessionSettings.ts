export interface SessionReconnectSettings {
  enabled: boolean;
  maxAttempts: number;
  intervalMs: number;
}

const STORAGE_KEY = 'session-reconnect-settings';

const DEFAULTS: SessionReconnectSettings = {
  enabled: true,
  maxAttempts: 3,
  intervalMs: 3000,
};

export function getSessionReconnectSettings(): SessionReconnectSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<SessionReconnectSettings>;
    return {
      enabled: parsed.enabled ?? DEFAULTS.enabled,
      maxAttempts: Math.min(10, Math.max(1, parsed.maxAttempts ?? DEFAULTS.maxAttempts)),
      intervalMs: Math.min(30000, Math.max(1000, parsed.intervalMs ?? DEFAULTS.intervalMs)),
    };
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveSessionReconnectSettings(settings: SessionReconnectSettings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}
