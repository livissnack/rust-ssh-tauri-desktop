export type ApiTab = 'http' | 'ws' | 'sse' | 'socketio' | 'mqtt';

export type LogDirection = 'in' | 'out' | 'system' | 'error';

export interface DebugLogEntry {
  id: string;
  time: string;
  direction: LogDirection;
  message: string;
}

export interface HeaderRow {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
}

export interface CommonHeader {
  key: string;
  values: string[];
}

export const COMMON_HTTP_HEADERS: CommonHeader[] = [
  { key: 'Accept', values: ['application/json', '*/*', 'text/plain', 'text/html'] },
  { key: 'Content-Type', values: ['application/json', 'application/x-www-form-urlencoded', 'multipart/form-data', 'text/plain'] },
  { key: 'Authorization', values: ['Bearer ', 'Basic '] },
  { key: 'User-Agent', values: ['hiphup-terminal/1.0'] },
  { key: 'Cache-Control', values: ['no-cache', 'no-store', 'max-age=0'] },
  { key: 'Accept-Language', values: ['zh-CN,zh;q=0.9', 'en-US,en;q=0.9'] },
  { key: 'Accept-Encoding', values: ['gzip, deflate, br'] },
  { key: 'Cookie', values: [] },
  { key: 'Origin', values: [] },
  { key: 'Referer', values: [] },
  { key: 'X-Requested-With', values: ['XMLHttpRequest'] },
  { key: 'X-Api-Key', values: [] },
];

export const COMMON_HEADER_KEYS = COMMON_HTTP_HEADERS.map((item) => item.key);

export function getHeaderValueSuggestions(key: string): string[] {
  const normalized = key.trim().toLowerCase();
  if (!normalized) return [];
  const match = COMMON_HTTP_HEADERS.find((item) => item.key.toLowerCase() === normalized);
  return match?.values ?? [];
}

export function createHeaderRow(key = '', value = ''): HeaderRow {
  return {
    id: Math.random().toString(36).slice(2, 9),
    key,
    value,
    enabled: true,
  };
}

export function parseHeaderRows(rows: HeaderRow[]): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const row of rows) {
    if (!row.enabled) continue;
    const key = row.key.trim();
    if (!key) continue;
    headers[key] = row.value;
  }
  return headers;
}

export function headersToText(headers: Record<string, string>): string {
  return Object.entries(headers)
    .map(([k, v]) => `${k}: ${v}`)
    .join('\n');
}

export function formatBodyPreview(body: string, max = 12000): string {
  if (body.length <= max) return body;
  return `${body.slice(0, max)}\n\n… [truncated ${body.length - max} chars]`;
}

export function tryFormatJson(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

export function nowTimeLabel(): string {
  return new Date().toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

export function createLog(direction: LogDirection, message: string): DebugLogEntry {
  return {
    id: Math.random().toString(36).slice(2, 11),
    time: nowTimeLabel(),
    direction,
    message,
  };
}
