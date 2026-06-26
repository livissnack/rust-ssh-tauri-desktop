import { invoke } from '@tauri-apps/api/core';
import { t } from './i18n.ts';

export interface AiChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
}

export interface AiChatSession {
  id: string;
  title: string;
  messages: AiChatMessage[];
  createdAt: number;
  updatedAt: number;
}

export interface AiChatSessionSummary {
  id: string;
  title: string;
  preview: string;
  messageCount: number;
  createdAt: number;
  updatedAt: number;
}

export const AI_CHAT_ACTIVE_SESSION_KEY = 'ai-chat-active-session-id';
export const AI_CHAT_RETENTION_DAYS = 7;

export function createAiChatSessionId(): string {
  return crypto.randomUUID();
}

export function formatAiChatTime(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) {
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
  return date.toLocaleString([], {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function buildSessionTitle(messages: AiChatMessage[]): string {
  const firstUser = messages.find((m) => m.role === 'user' && m.content.trim());
  if (!firstUser) return t('ai.newChat');
  const text = firstUser.content.replace(/\s+/g, ' ').trim();
  if (text.length <= 36) return text;
  return `${text.slice(0, 36)}…`;
}

export async function listAiChatSessions(): Promise<AiChatSessionSummary[]> {
  return invoke<AiChatSessionSummary[]>('list_ai_chat_sessions');
}

export async function getAiChatSession(sessionId: string): Promise<AiChatSession | null> {
  return invoke<AiChatSession | null>('get_ai_chat_session', { sessionId });
}

export async function saveAiChatSession(session: AiChatSession): Promise<AiChatSessionSummary> {
  return invoke<AiChatSessionSummary>('save_ai_chat_session', { session });
}

export async function deleteAiChatSession(sessionId: string): Promise<void> {
  await invoke('delete_ai_chat_session', { sessionId });
}

export async function clearAiChatSessions(): Promise<void> {
  await invoke('clear_ai_chat_sessions');
}

export async function pruneAiChatSessions(): Promise<number> {
  return invoke<number>('prune_ai_chat_sessions');
}

export function getActiveSessionId(): string | null {
  return localStorage.getItem(AI_CHAT_ACTIVE_SESSION_KEY);
}

export function setActiveSessionId(sessionId: string) {
  localStorage.setItem(AI_CHAT_ACTIVE_SESSION_KEY, sessionId);
}

export function clearActiveSessionId() {
  localStorage.removeItem(AI_CHAT_ACTIVE_SESSION_KEY);
}
