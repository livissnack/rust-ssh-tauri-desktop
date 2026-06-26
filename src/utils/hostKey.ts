import { invoke } from '@tauri-apps/api/core';

export type HostKeyRole = 'direct' | 'jump' | 'target';

export type HostKeyPromptBase = {
  requestId: string;
  hostRole: HostKeyRole;
  host: string;
  port: number;
  serverName: string;
  fingerprint: string;
  keyType: string;
};

export type HostKeyPromptNew = HostKeyPromptBase & {
  kind: 'new';
};

export type HostKeyPromptChanged = HostKeyPromptBase & {
  kind: 'changed';
  oldFingerprint: string;
};

export type HostKeyPrompt = HostKeyPromptNew | HostKeyPromptChanged;

function parseHostKeyRole(raw: unknown): HostKeyRole {
  const role = String(raw ?? 'direct').toLowerCase();
  if (role === 'jump') return 'jump';
  if (role === 'target') return 'target';
  return 'direct';
}

export function normalizeHostKeyPrompt(raw: Record<string, unknown>): HostKeyPrompt | null {
  const requestId = raw.requestId ?? raw.request_id;
  if (typeof requestId !== 'string' || !requestId) return null;

  const kind = String(raw.kind ?? '').toLowerCase();
  const base: HostKeyPromptBase = {
    requestId,
    hostRole: parseHostKeyRole(raw.hostRole ?? raw.host_role),
    host: String(raw.host ?? ''),
    port: Number(raw.port ?? 0),
    serverName: String(raw.serverName ?? raw.server_name ?? ''),
    fingerprint: String(raw.fingerprint ?? ''),
    keyType: String(raw.keyType ?? raw.key_type ?? ''),
  };

  if (kind === 'changed') {
    return {
      ...base,
      kind: 'changed',
      oldFingerprint: String(raw.oldFingerprint ?? raw.old_fingerprint ?? ''),
    };
  }

  return { ...base, kind: 'new' };
}

export async function respondToHostKeyPrompt(requestId: string, trust: boolean): Promise<void> {
  await invoke('respond_host_key_prompt', { requestId, trust });
}
