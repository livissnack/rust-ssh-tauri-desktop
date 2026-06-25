import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { createHeaderRow, type HeaderRow } from './apiDebugger.ts';

export type BodyType = 'none' | 'json' | 'text' | 'form';

export type RequestProtocol = 'http' | 'ws' | 'sse' | 'socketio' | 'mqtt';

export interface RequestSnapshot {
  protocol?: RequestProtocol;
  method: string;
  url: string;
  headers: Array<{ key: string; value: string; enabled: boolean }>;
  body: string;
  bodyType: BodyType;
  message?: string;
  path?: string;
  event?: string;
  payload?: string;
  clientId?: string;
  username?: string;
  password?: string;
  subTopic?: string;
  pubTopic?: string;
  pubMessage?: string;
}

/** @deprecated Use RequestSnapshot */
export type HttpRequestSnapshot = RequestSnapshot;

export interface SavedRequest {
  id: string;
  name: string;
  description?: string;
  snapshot: RequestSnapshot;
}

export interface ApiCollection {
  id: string;
  name: string;
  requests: SavedRequest[];
  updatedAt: number;
}

export interface EnvVariable {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
}

export interface ApiEnvironment {
  id: string;
  name: string;
  variables: EnvVariable[];
}

export interface HistoryEntry {
  id: string;
  timestamp: number;
  snapshot: RequestSnapshot;
  status?: number;
  elapsedMs?: number;
}

export interface ApiDebuggerStore {
  collections: ApiCollection[];
  environments: ApiEnvironment[];
  history: HistoryEntry[];
  activeEnvId: string | null;
}

const LEGACY_KEYS = {
  collections: 'api-debugger-collections',
  environments: 'api-debugger-environments',
  history: 'api-debugger-history',
  activeEnvId: 'api-debugger-active-env',
} as const;

export function createId(): string {
  return Math.random().toString(36).slice(2, 11);
}

function readLegacyJson<T>(key: string): T | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

function readLegacyStore(): ApiDebuggerStore | null {
  const collections = readLegacyJson<ApiCollection[]>(LEGACY_KEYS.collections);
  const environments = readLegacyJson<ApiEnvironment[]>(LEGACY_KEYS.environments);
  const history = readLegacyJson<HistoryEntry[]>(LEGACY_KEYS.history);
  const activeEnvId = localStorage.getItem(LEGACY_KEYS.activeEnvId);

  if (!collections?.length && !environments?.length && !history?.length && !activeEnvId) {
    return null;
  }

  return {
    collections: collections ?? [],
    environments: environments?.length
      ? environments
      : [{ id: createId(), name: 'Default', variables: [] }],
    history: history ?? [],
    activeEnvId,
  };
}

function clearLegacyStore() {
  Object.values(LEGACY_KEYS).forEach((key) => localStorage.removeItem(key));
}

export async function loadStore(): Promise<ApiDebuggerStore> {
  let store = await invoke<ApiDebuggerStore>('get_api_debugger_data');
  const legacy = readLegacyStore();
  if (legacy) {
    store = {
      collections: legacy.collections.length ? legacy.collections : store.collections,
      environments: legacy.environments.length ? legacy.environments : store.environments,
      history: legacy.history.length ? legacy.history : store.history,
      activeEnvId: legacy.activeEnvId ?? store.activeEnvId ?? store.environments[0]?.id ?? null,
    };
    await invoke('save_api_debugger_data', { store });
    clearLegacyStore();
  }
  if (!store.activeEnvId && store.environments[0]) {
    store.activeEnvId = store.environments[0].id;
  }
  return store;
}

export async function saveStore(store: ApiDebuggerStore): Promise<void> {
  await invoke('save_api_debugger_data', { store });
}

export async function pushHistory(entry: Omit<HistoryEntry, 'id'>, store: ApiDebuggerStore): Promise<ApiDebuggerStore> {
  const next: ApiDebuggerStore = {
    ...store,
    history: [{ ...entry, id: createId() }, ...store.history].slice(0, 100),
  };
  await saveStore(next);
  return next;
}

export type ExportFormat = 'hiphup' | 'postman-collection' | 'postman-environment';
export type ImportFormat = ExportFormat | 'auto';
export type ImportMode = 'merge' | 'replace';

const EXPORT_FILTERS: Record<ExportFormat, { name: string; extensions: string[]; defaultPath: string }> = {
  hiphup: { name: 'Hiphup API Backup', extensions: ['json'], defaultPath: 'hiphup-api-backup.json' },
  'postman-collection': { name: 'Postman Collection', extensions: ['json'], defaultPath: 'collection.postman_collection.json' },
  'postman-environment': { name: 'Postman Environment', extensions: ['json'], defaultPath: 'environment.postman_environment.json' },
};

export async function exportApiDebuggerData(format: ExportFormat): Promise<boolean> {
  const filter = EXPORT_FILTERS[format];
  const path = await save({
    title: '导出 API 数据',
    filters: [{ name: filter.name, extensions: filter.extensions }],
    defaultPath: filter.defaultPath,
  });
  if (!path) return false;
  await invoke('export_api_debugger_file', { path, format });
  return true;
}

export async function importApiDebuggerData(
  format: ImportFormat,
  mode: ImportMode,
): Promise<ApiDebuggerStore | null> {
  const path = await open({
    title: '导入 API 数据',
    filters: [{ name: 'JSON', extensions: ['json'] }],
    multiple: false,
  });
  if (!path || typeof path !== 'string') return null;
  return invoke<ApiDebuggerStore>('import_api_debugger_file', { path, format, mode });
}

export function getSnapshotProtocol(snapshot: RequestSnapshot): RequestProtocol {
  return snapshot.protocol ?? 'http';
}

export function protocolLabel(protocol: RequestProtocol): string {
  const labels: Record<RequestProtocol, string> = {
    http: 'HTTP',
    ws: 'WS',
    sse: 'SSE',
    socketio: 'SIO',
    mqtt: 'MQTT',
  };
  return labels[protocol];
}

export function snapshotTagLabel(snapshot: RequestSnapshot): string {
  const protocol = getSnapshotProtocol(snapshot);
  if (protocol === 'http') return snapshot.method.toUpperCase() || 'HTTP';
  return protocolLabel(protocol);
}

export function toSnapshot(
  method: string,
  url: string,
  headers: HeaderRow[],
  body: string,
  bodyType: BodyType,
): RequestSnapshot {
  return {
    protocol: 'http',
    method,
    url,
    body,
    bodyType,
    headers: headers.map(({ key, value, enabled }) => ({ key, value, enabled })),
  };
}

export function toWsSnapshot(url: string, message: string): RequestSnapshot {
  return {
    protocol: 'ws',
    method: 'WS',
    url,
    message,
    headers: [],
    body: '',
    bodyType: 'none',
  };
}

export function toSseSnapshot(url: string): RequestSnapshot {
  return {
    protocol: 'sse',
    method: 'SSE',
    url,
    headers: [],
    body: '',
    bodyType: 'none',
  };
}

export function toSocketIoSnapshot(
  url: string,
  path: string,
  event: string,
  payload: string,
): RequestSnapshot {
  return {
    protocol: 'socketio',
    method: 'SIO',
    url,
    path,
    event,
    payload,
    headers: [],
    body: '',
    bodyType: 'none',
  };
}

export function toMqttSnapshot(
  url: string,
  clientId: string,
  username: string,
  password: string,
  subTopic: string,
  pubTopic: string,
  pubMessage: string,
): RequestSnapshot {
  return {
    protocol: 'mqtt',
    method: 'MQTT',
    url,
    clientId,
    username,
    password,
    subTopic,
    pubTopic,
    pubMessage,
    headers: [],
    body: '',
    bodyType: 'none',
  };
}

export function fromSnapshot(snapshot: RequestSnapshot) {
  return {
    method: snapshot.method,
    url: snapshot.url,
    body: snapshot.body,
    bodyType: snapshot.bodyType,
    headers: snapshot.headers.map((h) => {
      const row = createHeaderRow(h.key, h.value);
      row.enabled = h.enabled;
      return row;
    }),
  };
}

export function applyEnvironmentVariables(text: string, variables: EnvVariable[]): string {
  if (!text.includes('{{')) return text;
  return text.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (match, key: string) => {
    const variable = variables.find((item) => item.enabled && item.key === key.trim());
    return variable ? variable.value : match;
  });
}

export function resolveHeaders(
  headers: HeaderRow[],
  variables: EnvVariable[],
): Record<string, string> {
  const result: Record<string, string> = {};
  for (const row of headers) {
    if (!row.enabled) continue;
    const key = applyEnvironmentVariables(row.key.trim(), variables);
    if (!key) continue;
    result[key] = applyEnvironmentVariables(row.value, variables);
  }
  return result;
}

export function formatHistoryTime(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();
  const isToday = date.toDateString() === now.toDateString();
  const time = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  if (isToday) return time;
  return `${date.toLocaleDateString([], { month: '2-digit', day: '2-digit' })} ${time}`;
}

export function createEnvVariable(key = '', value = ''): EnvVariable {
  return { id: createId(), key, value, enabled: true };
}

export function defaultRequestName(snapshot: RequestSnapshot): string {
  const protocol = getSnapshotProtocol(snapshot);
  const prefix = protocol === 'http'
    ? snapshot.method.toUpperCase()
    : protocolLabel(protocol);
  try {
    const parsed = new URL(snapshot.url);
    const segments = parsed.pathname.split('/').filter(Boolean);
    const last = segments[segments.length - 1];
    if (last) return `${prefix} · ${decodeURIComponent(last)}`;
    return `${prefix} · ${parsed.host}`;
  } catch {
    const trimmed = snapshot.url.trim();
    if (trimmed) return `${prefix} · ${trimmed.slice(0, 40)}`;
  }
  if (protocol === 'mqtt' && snapshot.subTopic) {
    return `${prefix} · ${snapshot.subTopic}`;
  }
  if (protocol === 'socketio' && snapshot.event) {
    return `${prefix} · ${snapshot.event}`;
  }
  return prefix;
}

export function snapshotPreviewText(snapshot: RequestSnapshot): string {
  const protocol = getSnapshotProtocol(snapshot);
  switch (protocol) {
    case 'ws':
      return snapshot.message ? `默认消息: ${snapshot.message}` : '';
    case 'socketio':
      return [
        snapshot.path ? `Path: ${snapshot.path}` : '',
        snapshot.event ? `Event: ${snapshot.event}` : '',
        snapshot.payload ? `Payload: ${snapshot.payload}` : '',
      ].filter(Boolean).join(' · ');
    case 'mqtt':
      return [
        snapshot.subTopic ? `Sub: ${snapshot.subTopic}` : '',
        snapshot.pubTopic ? `Pub: ${snapshot.pubTopic}` : '',
      ].filter(Boolean).join(' · ');
    default:
      return '';
  }
}

export function buildStore(
  collections: ApiCollection[],
  environments: ApiEnvironment[],
  history: HistoryEntry[],
  activeEnvId: string | null,
): ApiDebuggerStore {
  return { collections, environments, history, activeEnvId };
}
